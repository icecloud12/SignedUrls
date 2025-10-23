pub enum File {
    ID,
    FILE_NAME,
    PATH,
    PROJECT_ID,
    REQUEST_ID,
    OPTIONS
}

impl ToString for File {
    fn to_string(&self)-> String {
        match &self {
            Self::ID => "_id".to_string(),
            Self::FILE_NAME => "file_name".to_string(),
            Self::PATH => "path".to_string(),
            Self::PROJECT_ID => "project_id".to_string(),
            Self::REQUEST_ID => "request_id".to_string(),
            Self::OPTIONS => "options".to_string()
        }
    }
}

impl From<File> for String {
    fn from(value: File) -> Self {
        value.to_string()
    }
}
