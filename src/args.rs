pub struct Args {
    pub(crate) server_address: String,
    pub(crate) bind_address: String,
}

pub fn parse_args() -> Result<Args, pico_args::Error> {
    let mut pargs = pico_args::Arguments::from_env();

    let mut bind_address = "0.0.0.0:0".to_string();
    match pargs.value_from_str("--bind_address") {
        Ok(val) => bind_address = val,
        Err(_) => {}
    }
    let args = Args {
        // Parses a required value that implements `FromStr`.
        // Returns an error if not present.
        server_address: pargs.value_from_str("--server_address")?,
        bind_address,
        // Parses an optional value that implements `FromStr`.
    };
    let remaining = pargs.finish();
    if !remaining.is_empty() {
        eprintln!("Warning: unused arguments left: {:?}.", remaining);
    }

    Ok(args)
}
