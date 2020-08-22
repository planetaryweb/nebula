use regex::Regex;

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn it_works() {
	}
}

pub(crate) struct StringValidator {
    pub min_len: usize,
    pub max_len: usize,
    pub regex: Regex,
}
