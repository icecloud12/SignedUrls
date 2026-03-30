pub enum ViewFileRequest {
    ID,
    REQUEST_ID,
    FILE_ID,
    OPTIONS
}

impl ToString for ViewFileRequest {
    fn to_string(&self) -> String {
        match &self {
            Self::ID => "_id".to_string(),
            Self::REQUEST_ID => "request_id".to_string(),
            Self::FILE_ID => "file_id".to_string(),
            Self::OPTIONS => "options".to_string()
        }
    }
}
impl From<ViewFileRequest> for String {
    fn from(value: ViewFileRequest) -> Self {
        value.to_string()
    }
}
