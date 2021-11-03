use std::collections::HashMap;
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

#[derive(Debug)]
struct VIn {
    id: u32,
    txdir: String,
    vout: u32,
    firm: String,
    pub_key: String,
}

#[derive(Debug)]
struct VOut {
    id: u32,
    txdir: String,
    username: String,
    hashed_pub_key: String,
    value: u32,
}

#[derive(Debug)]
struct Transaction {
    id: String,
    vins: Vec<VIn>,
    vouts: Vec<VOut>,
    is_coinbase: bool,
}

impl Transaction {
    fn new(id: String, vins: Vec<VIn>, vouts: Vec<VOut>, is_coinbase: bool) -> Transaction {
        Transaction {
            id,
            vins,
            vouts,
            is_coinbase,
        }
    }
}

//
// Manager of transactions
//

#[derive(Debug)]
struct TransactionManager {
    transactions: Vec<Transaction>,
    counter: u32,
}

impl TransactionManager {
    fn new() -> TransactionManager {
        TransactionManager {
            transactions: Vec::new(),
            counter: 0,
        }
    }

    fn create_coinbase(&mut self, user: &User, value: u32) {
        let mut vins = Vec::new();
        let mut vouts = Vec::new();
        let txdir = self.counter;

        let vin = VIn {
            id: 0,
            txdir: "".to_string(),
            vout: 0,
            firm: "".to_string(),
            pub_key: "".to_string(),
        };

        let vout = VOut {
            id: 0,
            txdir: txdir.to_string(),
            username: user.username.clone(),
            hashed_pub_key: user.pub_key.clone(),
            value,
        };

        vins.push(vin);
        vouts.push(vout);

        let tx = Transaction::new(txdir.to_string(), vins, vouts, true);
        self.counter += 1;

        self.transactions.push(tx);
    }

    fn get_unspent(&self, user: &User) -> Vec<&VOut> {
        let mut unspent_tx: Vec<&VOut> = Vec::new();
        let mut spent_txs: HashMap<String, Vec<u32>> = HashMap::new();

        for tx_index in (0..self.transactions.len()).rev() {
            let tx = &self.transactions[tx_index];

            'outs: for out in &tx.vouts {
                if spent_txs.contains_key(&tx.id) {
                    let spentout_by_tx = spent_txs.get(&tx.id).unwrap();
                    for spent_out in spentout_by_tx {
                        if spent_out == &out.id {
                            continue 'outs;
                        }
                    }
                }

                // Verificar que esa salida pertenece al usuario
                if out.hashed_pub_key == user.pub_key {
                    unspent_tx.push(out.clone());
                }
            }

            if tx.is_coinbase == false {
                for vin in &tx.vins {
                    if vin.pub_key == user.pub_key {
                        let spentout_by_tx = spent_txs.entry(vin.txdir.clone()).or_default();
                        spentout_by_tx.push(vin.vout);
                    }
                }
            }
        }

        unspent_tx.clone()
    }

    fn get_balance(&self, user: &User) -> u32 {
        let unspents = self.get_unspent(user);

        self.get_balance_in_vout(unspents)
    }

    fn get_balance_in_vout(&self, vouts: Vec<&VOut>) -> u32 {
        let mut balance = 0;
        for out in vouts {
            balance += out.value;
        }

        balance
    }

    fn send(&mut self, from: &User, to: &User, value: u32) {
        let unspent = self.get_unspent(from);
        let mut vins = Vec::new();
        let mut total = 0;
        let mut vouts = Vec::new();
        let new_txdir = self.counter;

        for i in 0..unspent.len() {
            let out = unspent[i];

            total += out.value;

            vins.push(VIn {
                id: i as u32,
                txdir: out.txdir.clone(),
                vout: out.id,
                firm: "".to_string(),
                pub_key: from.pub_key.clone(),
            });

            if total >= value {
                break;
            }
        }

        vouts.push(VOut {
            id: 0,
            txdir: new_txdir.to_string(),
            username: to.username.clone(),
            hashed_pub_key: to.pub_key.clone(),
            value,
        });

        if total > value {
            vouts.push(VOut {
                id: 1,
                txdir: new_txdir.to_string(),
                username: from.username.clone(),
                hashed_pub_key: from.pub_key.clone(),
                value: (total as i64 - value as i64) as u32,
            })
        }

        let new_transaction = Transaction::new(new_txdir.to_string(), vins, vouts, false);
        self.counter += 1;

        self.transactions.push(new_transaction);
    }

    fn print_transactions(&self) {
        for tx in &self.transactions {
            println!("Transaction {}", tx.id);
            println!(" Ins:");
            for ins in &tx.vins {
                println!("  id: {}", ins.id);
                println!("  txdir: {}", ins.txdir);
                println!("  firm: {}", ins.firm);
                println!("  pub_key: {}", ins.pub_key);
                println!("  vout: {}", ins.vout);
                println!("-");
            }
            println!(" Outs:");
            for outs in &tx.vouts {
                println!("  id: {}", outs.id);
                println!("  txdir: {}", outs.txdir);
                println!("  username: {}", outs.username);
                println!("  pub_key: {}", outs.hashed_pub_key);
                println!("  value: {}", outs.value);
                println!("-");
            }
        }
    }
}

fn main() {
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

    let mut transaction_manager = TransactionManager::new();
    transaction_manager.create_coinbase(&user_juan, 20);

    get_balance(&transaction_manager, &user_juan);
    get_balance(&transaction_manager, &user_pedro);

    transaction_manager.send(&user_juan, &user_pedro, 10);

    get_balance(&transaction_manager, &user_juan);
    get_balance(&transaction_manager, &user_pedro);

    transaction_manager.print_transactions();
    transaction_manager.send(&user_pedro, &user_juan, 3);

    get_balance(&transaction_manager, &user_juan);
    get_balance(&transaction_manager, &user_pedro);

    transaction_manager.print_transactions();
    transaction_manager.send(&user_juan, &user_pedro, 10);

    get_balance(&transaction_manager, &user_juan);
    get_balance(&transaction_manager, &user_pedro);

    transaction_manager.print_transactions();
}

fn get_balance(transaction_manager: &TransactionManager, user: &User) {
    let balance_juan = transaction_manager.get_balance(&user);
    println!("saldo de {}: {}", user.username, balance_juan);
}
