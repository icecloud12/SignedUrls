pub enum Bucket {
    ID,
    NAME,
    PUBLIC_KEY,
    HASHED_SECRET_KEY,
    SECRET_KEY_HINT
}

impl ToString for Bucket {
    fn to_string(&self)-> String {
        match self {
            Self::ID => String::from("_id"),
            Self::NAME => String::from("name"),
            Self::PUBLIC_KEY => String::from("public_key"),
            Self::HASHED_SECRET_KEY => String::from("hashed_secret_key"),
            Self::SECRET_KEY_HINT => String::from("secret_key_hint")
        }
    }
}

impl From<Bucket> for String {
    fn from(value: Bucket) -> Self {
        value.to_string()
    }
}
