pub enum MongoDbCollection{
    SampleCollection,
}

impl ToString for MongoDbCollection {
    fn to_string(&self) -> String{
        match self {
            Self::SampleCollection => String::from("SampleCollection")
        }
    }
}