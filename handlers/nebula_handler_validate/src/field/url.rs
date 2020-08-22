#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
    }
}

pub(crate) struct UrlValidator {
    pub valid_domains: Option<Vec<String>>,
}
