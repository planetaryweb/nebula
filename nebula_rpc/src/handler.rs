
#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn it_works() {
	}
}

type Handlers = HashMap<String, Handler>;

pub struct Handler {
    plugin: String,
    config: Config,
}
