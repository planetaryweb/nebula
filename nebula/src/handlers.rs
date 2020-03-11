use nebula_email;

enum Handler {
    #[cfg(features = "email")]
    Email(nebula_email::Config),
}
