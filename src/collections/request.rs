use mongodb::bson::Bson;

pub enum Request {
    ID,
    PROJECT_ID,
    DATE_CREATED,
    EXPIRATION_DATE,
    PERMISSION,
    FILES,
    OPTIONS,
}

impl ToString for Request{
    fn to_string(&self)->String{
        match &self {
            Self::ID => "_id".to_string(),
            Self::PROJECT_ID => "project_id".to_string(),
            Self::DATE_CREATED => "date_created".to_string(),
            Self::EXPIRATION_DATE => "expiration_date".to_string(),
            Self::PERMISSION => "permission".to_string(),
            Self::FILES => "files".to_string(),
            Self::OPTIONS => "options".to_string()
        }
    }
}
impl From<Request> for String {
    fn from(value: Request) -> Self {
        value.to_string()
    }
}

impl From<Request> for Bson {
    fn from(value: Request) -> Self {
        Bson::String(value.to_string())
    }
}
