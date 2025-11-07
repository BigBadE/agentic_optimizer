pub struct User {
    pub id: u64,
    pub name: String,
    pub email: String,
}

pub struct Post {
    pub id: u64,
    pub title: String,
    pub content: String,
    pub author_id: u64,
}
