# Nebula_Form

`nebula_form` is a small Rust library that provides a simple interface for
working with form data.

## Features

- Parse `application/x-www-form-urlencoded` and `multipart/form-data` forms
  from request bodies (currently only for `warp`).
- A `Form` object that can be manipulated (fields added, removed, etc.)
- Create `application/x-www-form-urlencoded` and `multipart/form-data` request
  bodies from a `Form` object.

## Non-Features

- Multiple values for a single form field
  - Note that there is no standard way to do this.
  
## Usage

```rust
use nebula_form::{Form, Field};
use warp::Filter;

fn main() {
    let form = Form::new();
    form.insert("field-foo", Field::Text(String::from("contents")));

    // Don't use `panic!` in actual code
    match form.get("field-foo") {
        None => panic!("Field expected"),
        Some(field) => {
            match field {
                Field::Text(txt) => println!(txt),
                Field::File(_) => panic!("This should not be a file!"),
            }
        }
    }

    // `make_request` doesn't actually exist and stands in for any usual way
    // of creating an HTTP request.
    make_request("POST", form.to_url_encoded().as_bytes());
    make_request("POST", form.to_multipart_bytes());

    // When using warp, the `form_filter` function parses the request body into
    // a `Form`.
    let hi = warp::path("some-form")
        .and(warp::method::post())
        .and(nebula_form::form_filter())
        .map(|form: Form| {
            format!("Hello {}!", form.get("name").unwrap())
        });
}
```