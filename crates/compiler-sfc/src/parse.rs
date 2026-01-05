#[derive(Debug, Default)]
pub struct SFCParseOptions {
    filename: Option<String>,
}

#[derive(Debug)]
pub struct SFCDescriptor {
    pub filename: String,
    pub source: String,
}

#[derive(Debug)]
pub struct SFCParseResult {
    pub descriptor: SFCDescriptor,
}

pub fn parse(source: String, options: Option<SFCParseOptions>) -> SFCParseResult {
    let SFCParseOptions { filename } = options.unwrap_or_default();

    todo!()
}
