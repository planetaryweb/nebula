use nebula_rpc::server::Handler;
use std::collections::HashMap;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}

mod captcha;
mod field;

struct Validator {
    //pub captcha: Option<Captcha>,
    pub fields: HashMap<String, field::Type>,
}

pub struct ValidateHandler {}

//impl Handler for ValidateHandler {
//    async fn handle(&self, config: nebula_rpc::Config, form: Form) -> Status<Bytes> {
//        
//    }
//    async fn validate(&self, config: nebula_rpc::Config) -> Status<Bytes> {
//        todo!()
//    }
//}
