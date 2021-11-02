// Bank Transactions as blockchain TX

//
// User
//

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

struct VIn {
    id: u32,
    txdir: String,
    vout: u32,
    firm: String,
    pub_key: String,
}

struct VOut {
    id: u32,
    username: String,
    hashed_pub_key: String,
    value: u32,
}

struct Transaction {
    id: String,
    vins: Vec<VIn>,
    vouts: Vec<VOut>,
}

impl Transaction {
    fn new(id: String, vins: Vec<VIn>, vouts: Vec<VOut>) -> Transaction {
        Transaction {
            id,
            vins,
            vouts,
        }
    }
}

//
// Manager of transactions
//

struct TransactionManager {
    transactions: Vec<Transaction>,
}

impl TransactionManager {
    fn new() -> TransactionManager {
        TransactionManager {
            transactions: Vec::new(),
        }
    }

    fn create_coinbase(&mut self, user: &User, value: u32) {
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
            hashed_pub_key: "".to_string(),
            value,
        };

        vins.push(vin);
        vouts.push(vout);

        let tx = Transaction::new(
            "".to_string(),
            vins,
            vouts,
        );

        self.transactions.push(tx);
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
}