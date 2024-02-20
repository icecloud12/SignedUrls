pub struct HTTPRequest {
    pub protocol: String,
    pub path: String,
    pub version: String, // ??
}

pub  fn wrap (line:String) -> HTTPRequest{
    let splitted_a:Vec<String>= line.split_whitespace().map(str::to_string).collect();
    
    HTTPRequest {
        protocol: splitted_a[0].to_owned(),
        path: splitted_a[1].to_owned(),
        version: splitted_a[2].to_owned(),
    }
}

pub enum ProtocolCollection {
    GET,
    POST,
    PUT,
    PATCH
}

impl ToString for ProtocolCollection{
    fn to_string(&self) -> String {
        match self {
            Self::GET => String::from("GET"),
            Self::POST => String::from("POST"),
            Self::PUT => String::from("PUT"),
            Self::PATCH => String::from("PATCH"),
        }
    }
}
pub struct Route
{
    pub protocol: String,
    pub route: String,
    pub handler: Box<dyn FnOnce()>
}
pub struct Router
{
    pub routes: Vec<Route>,
}
impl Router {
    pub fn new() -> Self{
        Self {
            routes: vec![]
        }
    }

    fn add_route(&mut self, path:&str, protocol:String, handler: impl FnOnce() + 'static)
    {
        let route:Route = Route {
            protocol,
            route: path.to_string(),
            handler: Box::new(handler)
        };
        self.routes.push(route);
    }

    pub fn get(&mut self, path:&str, handler:impl FnOnce() + 'static)
    {
        self.add_route(path, ProtocolCollection::GET.to_string(),handler);
    }
    
    pub fn post(&mut self, path:&str, handler: impl FnOnce() +'static)
    {
        self.add_route(path, ProtocolCollection::POST.to_string(), handler);
    }
    
    pub fn put(&mut self, path:&str, handler: impl FnOnce() +'static)
    {
        self.add_route(path, ProtocolCollection::PUT.to_string(), handler);
    }

    pub fn patch(&mut self, path:&str, handler: impl FnOnce() +'static)
    {
        self.add_route(path, ProtocolCollection::PATCH.to_string(), handler);
    }
    
}


