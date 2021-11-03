use futures::stream::TryStreamExt;
use sha2::{Digest, Sha256};
use std::{collections::HashMap, time::SystemTime};

use mongodb::{bson::doc, options::ClientOptions, options::FindOptions, Client};
use serde::{Deserialize, Serialize};

// Bank Transactions as blockchain TX

//
// User
//

#[derive(Debug)]
struct User {
    username: String,
    pub_key: String,
    priv_key: String,
}

impl User {
    fn new(username: String, pub_key: String, priv_key: String) -> User {
        User {
            username,
            pub_key,
            priv_key,
        }
    }
}

//
// System of transactions with VIn and VOut
//

#[derive(Debug, Clone, Serialize, Deserialize)]
struct VIn {
    id: u32,
    txdir: String,
    vout: u32,
    firm: String,
    pub_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct VOut {
    id: u32,
    username: String,
    hashed_pub_key: String,
    value: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Transaction {
    hash: String,
    timestamp: u64,
    vins: Vec<VIn>,
    vouts: Vec<VOut>,
    users: Vec<String>,
    is_coinbase: bool,
}

impl Transaction {
    fn new(vins: Vec<VIn>, vouts: Vec<VOut>, users: Vec<String>, is_coinbase: bool) -> Transaction {
        let mut hasher = Sha256::new();

        let timestamp = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        hasher.update(timestamp.to_string().as_bytes());

        vins.iter().for_each(|vin| {
            hasher.update(format!("{}", vouts.len()).as_bytes());
            hasher.update(format!("{}", vin.id).as_bytes());
            hasher.update(format!("{}", vin.txdir).as_bytes());
            hasher.update(format!("{}", vin.vout).as_bytes());
            hasher.update(format!("{}", vin.firm).as_bytes());
            hasher.update(format!("{}", vin.pub_key).as_bytes());
        });

        vouts.iter().for_each(|vout| {
            hasher.update(format!("{}", vouts.len()).as_bytes());
            hasher.update(format!("{}", vout.id).as_bytes());
            hasher.update(format!("{}", vout.hashed_pub_key).as_bytes());
            hasher.update(format!("{}", vout.username).as_bytes());
            hasher.update(format!("{}", vout.value).as_bytes());
        });

        users.iter().for_each(|user| {
            hasher.update(format!("{}", users.len()).as_bytes());
            hasher.update(format!("{}", user).as_bytes());
        });

        Transaction {
            hash: format!("{:x}", hasher.finalize()),
            timestamp,
            vins,
            vouts,
            is_coinbase,
            users,
        }
    }
}

//
// Manager of transactions
//

#[derive(Debug)]
struct TransactionManager {
    db: mongodb::Database,
}

impl TransactionManager {
    async fn new() -> Result<TransactionManager, mongodb::error::Error> {
        let opts = ClientOptions::parse("mongodb://localhost:27017/").await?;
        let client = Client::with_options(opts)?;

        Ok(TransactionManager {
            db: client.database("ressy"),
        })
    }

    async fn create_coinbase(
        &mut self,
        user: &User,
        value: u32,
    ) -> Result<(), mongodb::error::Error> {
        let tx_coll = self.db.collection::<Transaction>("transactions");

        let mut vins = Vec::new();
        let mut vouts = Vec::new();

        let vin = VIn {
            id: 0,
            txdir: "".to_string(),
            vout: 0,
            firm: "".to_string(),
            pub_key: "".to_string(),
        };

        let vout = VOut {
            id: 0,
            username: user.username.clone(),
            hashed_pub_key: user.pub_key.clone(),
            value,
        };

        vins.push(vin);
        vouts.push(vout);

        let tx = Transaction::new(vins, vouts, vec![user.pub_key.to_string()], true);
        tx_coll.insert_one(tx, None).await?;

        Ok(())
    }

    async fn get_unspent<'a>(
        &self,
        user: &User,
    ) -> Result<Vec<(String, VOut)>, mongodb::error::Error> {
        let tx_coll = self.db.collection::<Transaction>("transactions");

        let filter = doc! {"users": user.pub_key.clone()};
        let options = FindOptions::builder()
            .sort(doc! {"$natural": -1})
            .build();

        let mut transactions = tx_coll.find(filter, options).await?;

        let mut unspent_tx: Vec<(String, VOut)> = Vec::new();
        let mut spent_txs: HashMap<String, Vec<u32>> = HashMap::new();

        while let Some(transaction) = transactions.try_next().await? {
            'outs: for out in &transaction.vouts {
                if spent_txs.contains_key(&transaction.hash) {
                    let spentout_by_tx = spent_txs.get(&transaction.hash).unwrap();
                    for spent_out in spentout_by_tx {
                        if spent_out == &out.id {
                            continue 'outs;
                        }
                    }
                }

                // Verificar que esa salida pertenece al usuario
                if out.hashed_pub_key == user.pub_key {
                    unspent_tx.push((transaction.hash.clone(), out.clone()));
                }
            }

            if transaction.is_coinbase == false {
                for vin in &transaction.vins {
                    if vin.pub_key == user.pub_key {
                        let spentout_by_tx = spent_txs.entry(vin.txdir.clone()).or_default();
                        spentout_by_tx.push(vin.vout);
                    }
                }
            }
        }

        Ok(unspent_tx)
    }

    async fn get_balance(&self, user: &User) -> Result<u32, mongodb::error::Error> {
        let transaction_unspent = self.get_unspent(user).await?;

        let mut balance = 0;

        for (_, vout) in transaction_unspent {
            if vout.hashed_pub_key == user.pub_key {
                balance += vout.value;
            }
        }

        Ok(balance)
    }

    async fn send(&mut self, from: &User, to: &User, value: u32) {
        let tx_coll = self.db.collection::<Transaction>("transactions");

        let transactions_unspent = self.get_unspent(from).await.unwrap();
        let mut vins = Vec::new();
        let mut total = 0;
        let mut vouts = Vec::new();

        'txs: for i in 0..transactions_unspent.len() {
            let (hash, vout) = &transactions_unspent[i];
            if vout.hashed_pub_key == from.pub_key {
                total += vout.value;

                vins.push(VIn {
                    id: i as u32,
                    txdir: hash.clone(),
                    vout: vout.id,
                    firm: "".to_string(),
                    pub_key: from.pub_key.clone(),
                });

                if total >= value {
                    break 'txs;
                }
            }
        }

        if total < value {
            panic!("Not enough money");
        }

        vouts.push(VOut {
            id: 0,
            username: to.username.clone(),
            hashed_pub_key: to.pub_key.clone(),
            value,
        });

        if total > value {
            vouts.push(VOut {
                id: 1,
                username: from.username.clone(),
                hashed_pub_key: from.pub_key.clone(),
                value: (total as i64 - value as i64) as u32,
            })
        }

        let new_transaction = Transaction::new(
            vins,
            vouts,
            vec![from.pub_key.clone(), to.pub_key.clone()],
            false,
        );
        tx_coll.insert_one(new_transaction, None).await.unwrap();

        println!(
            "{} envio {} dolares a {}",
            from.username, value, to.username
        );
    }
}

#[tokio::main]
async fn main() -> Result<(), mongodb::error::Error> {
    let user_juan = User::new(
        "juan".to_string(),
        "juan123".to_string(),
        "j123".to_string(),
    );
    let user_pedro = User::new(
        "pedro".to_string(),
        "pedro123".to_string(),
        "p123".to_string(),
    );

    let user_yeff = User::new(
        "Yeferson".to_string(),
        "yeff123".to_string(),
        "y123".to_string(),
    );

    let mut transaction_manager = TransactionManager::new().await?;
    // transaction_manager.create_coinbase(&user_juan, 20).await?;

    get_balance(&transaction_manager, &user_juan).await;
    get_balance(&transaction_manager, &user_pedro).await;
    get_balance(&transaction_manager, &user_yeff).await;

    // // transaction_manager.send(&user_juan, &user_pedro, 8).await;
    // transaction_manager.send(&user_pedro, &user_juan, 5).await;
    // transaction_manager.send(&user_juan, &user_pedro, 10).await;
    // transaction_manager.send(&user_pedro, &user_yeff, 10).await;
    // transaction_manager.send(&user_yeff, &user_pedro, 7).await;

    // transaction_manager.print_transactions();

    // get_balance(&transaction_manager, &user_juan);
    // get_balance(&transaction_manager, &user_pedro);
    // get_balance(&transaction_manager, &user_yeff);

    Ok(())
}

async fn get_balance(transaction_manager: &TransactionManager, user: &User) {
    let balance_juan = transaction_manager.get_balance(&user).await.unwrap();
    println!("saldo de {}: {}", user.username, balance_juan);
}
