
// Stolen from syn crate.
pub fn unwrap<T>(name: &'static str,
             f: fn(&str) -> synom::IResult<&str, T>,
             input: &str)
             -> Result<T, String> {
    match f(input) {
        synom::IResult::Done(mut rest, t) => {
            rest = synom::space::skip_whitespace(rest);
            if rest.is_empty() {
                Ok(t)
            } else if rest.len() == input.len() {
                // parsed nothing
                Err(format!("failed to parse {}: {:?}", name, rest))
            } else {
                Err(format!("unparsed tokens after {}: {:?}", name, rest))
            }
        }
        synom::IResult::Error => Err(format!("failed to parse {}: {:?}", name, input)),
    }
}
