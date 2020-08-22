
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
    }
}

pub(crate) struct PhoneValidator {
    pub valid_area_codes: Option<Vec<String>>,
}
