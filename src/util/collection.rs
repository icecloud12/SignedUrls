pub enum MongoDbCollection{
    SampleCollection,
}

impl MongoDbCollection {
    fn as_str(&self) -> &'static str{
        match self {
            MongoDbCollection::SampleCollection => "SampleCollection"
        }
    }
}