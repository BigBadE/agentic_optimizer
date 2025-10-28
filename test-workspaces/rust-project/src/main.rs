use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
struct User {
    id: u64,
    name: String,
    email: String,
}

impl User {
    fn new(id: u64, name: String, email: String) -> Self {
        Self { id, name, email }
    }

    fn display(&self) -> String {
        format!("{} ({})", self.name, self.email)
    }
}

#[tokio::main]
async fn main() {
    let user = User::new(1, "Alice".to_string(), "alice@example.com".to_string());
    println!("User: {}", user.display());
}
