use crate::{ast::Position, errors::ErrorCodes, parser::ParserContext};

#[derive(Debug, PartialEq, Clone)]
pub enum ParseMode {
    BASE,
    HTML,
    SFC,
}

#[derive(Debug, PartialEq, Clone, Copy)]
#[repr(u32)]
pub enum CharCodes {
    /// "\t"
    Tab = 0x9,
    /// "\n"
    NewLine = 0xa,
    /// "\f"
    FormFeed = 0xc,
    /// "\r"
    CarriageReturn = 0xd,
    // " "
    Space = 0x20,
    /// "!"
    ExclamationMark = 0x21,
    /// "#"
    Number = 0x23,
    /// "&"
    Amp = 0x26,
    /// "'"
    SingleQuote = 0x27,
    /// '"'
    DoubleQuote = 0x22,
    /// "`"
    GraveAccent = 96,
    /// "-"
    Dash = 0x2d,
    /// "/"
    Slash = 0x2f,
    /// "0"
    Zero = 0x30,
    /// "9"
    Nine = 0x39,
    /// ";"
    Semi = 0x3b,
    /// "<"
    Lt = 0x3c,
    /// "="
    Eq = 0x3d,
    /// ">"
    Gt = 0x3e,
    /// "?"
    Questionmark = 0x3f,
    /// "A"
    UpperA = 0x41,
    /// "a"
    LowerA = 0x61,
    /// "F"
    UpperF = 0x46,
    /// "f"
    LowerF = 0x66,
    /// "Z"
    UpperZ = 0x5a,
    /// "z"
    LowerZ = 0x7a,
    /// "x"
    LowerX = 0x78,
    /// "v"
    LowerV = 0x76,
    /// "."
    Dot = 0x2e,
    /// ":"
    Colon = 0x3a,
    /// "@"
    At = 0x40,
    /// "["
    LeftSquare = 91,
    /// "]"
    RightSquare = 93,
}

impl PartialEq<u32> for CharCodes {
    fn eq(&self, other: &u32) -> bool {
        *self as u32 == *other
    }
}

impl PartialEq<CharCodes> for u32 {
    fn eq(&self, other: &CharCodes) -> bool {
        *self == *other as u32
    }
}

impl PartialOrd<CharCodes> for u32 {
    fn partial_cmp(&self, other: &CharCodes) -> Option<std::cmp::Ordering> {
        let other = *other as u32;
        self.partial_cmp(&other)
    }
}

/** All the states the tokenizer can be in. */
#[derive(Debug, Clone, PartialEq)]
pub enum State {
    Text = 1,

    // interpolation
    InterpolationOpen,
    Interpolation,
    InterpolationClose,

    // Tags
    /// After <
    BeforeTagName,
    InTagName,
    InSelfClosingTag,
    BeforeClosingTagName,
    InClosingTagName,
    AfterClosingTagName,

    // Attrs
    BeforeAttrName,
    InAttrName,
    InDirName,
    InDirArg,
    InDirDynamicArg,
    InDirModifier,
    AfterAttrName,
    BeforeAttrValue,
    /// "
    InAttrValueDq,
    /// '
    InAttrValueSq,
    InAttrValueNq,

    // Declarations
    /// !
    BeforeDeclaration,
    InDeclaration,

    // Processing instructions
    /// ?
    InProcessingInstruction,

    // Comments & CDATA
    BeforeComment,
    CDATASequence,
    InSpecialComment,
    InCommentLike,

    // Special tags
    /// Decide if we deal with `<script` or `<style`
    BeforeSpecialS,
    /// Decide if we deal with `<title` or `<textarea`
    BeforeSpecialT,
    SpecialStartSequence,
    InRCDATA,

    InEntity,

    InSFCRootTagName,
}

/// HTML only allows ASCII alpha characters (a-z and A-Z) at the beginning of a
/// tag name.
fn is_tag_start_char(c: u32) -> bool {
    (c >= CharCodes::LowerA && c <= CharCodes::LowerZ)
        || (c >= CharCodes::UpperA && c <= CharCodes::UpperZ)
}

pub fn is_whitespace(c: u32) -> bool {
    c == CharCodes::Space
        || c == CharCodes::NewLine
        || c == CharCodes::Tab
        || c == CharCodes::FormFeed
        || c == CharCodes::CarriageReturn
}

fn is_end_of_tag_section(c: u32) -> bool {
    c == CharCodes::Slash || c == CharCodes::Gt || is_whitespace(c)
}

pub fn to_char_codes(str: String) -> Vec<u32> {
    str.chars().map(|c| c as u32).collect()
}

#[derive(Debug, PartialEq)]
pub enum QuoteType {
    NoValue,
    Unquoted,
    Single,
    Double,
}

/// Sequences used to match longer strings.
///
/// We don't have `Script`, `Style`, or `Title` here. Instead, we re-use the *End
/// sequences with an increased offset.
#[derive(Debug)]
pub struct Sequences {
    /// CDATA[
    cdata: Vec<u32>,
    /// ]]>
    pub cdata_end: Vec<u32>,
    /// `-->`
    comment_end: Vec<u32>,
    /// `</script`
    script_end: Vec<u32>,
    /// `</style`
    style_end: Vec<u32>,
    /// `</title`
    title_end: Vec<u32>,
    /// `</textarea
    textarea_end: Vec<u32>,
}

impl Sequences {
    fn new() -> Self {
        Self {
            cdata: vec![0x43, 0x44, 0x41, 0x54, 0x41, 0x5b],
            cdata_end: vec![0x5d, 0x5d, 0x3e],
            comment_end: vec![0x2d, 0x2d, 0x3e],
            script_end: vec![0x3c, 0x2f, 0x73, 0x63, 0x72, 0x69, 0x70, 0x74],
            style_end: vec![0x3c, 0x2f, 0x73, 0x74, 0x79, 0x6c, 0x65],
            title_end: vec![0x3c, 0x2f, 0x74, 0x69, 0x74, 0x6c, 0x65],
            textarea_end: vec![0x3c, 0x2f, 116, 101, 120, 116, 97, 114, 101, 97],
        }
    }
}

pub struct Tokenizer<'a> {
    /// The current state the tokenizer is in.
    pub state: State,
    /// The read buffer.
    pub buffer: Vec<char>,
    /// The beginning of the section that is currently being read.
    /// js type: -1 or usize
    pub section_start: Option<usize>,
    /// The index within the buffer that we are currently looking at.
    index: usize,
    /// The start of the last entity.
    entity_start: usize,
    /// Some behavior, eg. when decoding entities, is done while we are in another state. This keeps track of the other state type.
    base_state: State,
    /// For special parsing behavior inside of script and style tags.
    pub in_rc_data: bool,
    /// For disabling RCDATA tags handling
    pub in_xml: bool,
    /// For disabling interpolation parsing in v-pre
    pub in_v_pre: bool,
    /// Record newline positions for fast line / column calculation
    newlines: Vec<usize>,

    pub mode: ParseMode,

    pub delimiter_open: Vec<u32>,
    pub delimiter_close: Vec<u32>,
    /// js type: -1 or usize
    delimiter_index: Option<usize>,

    pub current_sequence: Vec<u32>,
    sequence_index: usize,

    pub context: ParserContext<'a>,
    pub sequences: Sequences,
}

impl<'a> Tokenizer<'a> {
    pub fn new(context: ParserContext<'a>) -> Self {
        Self {
            state: State::Text,
            buffer: Vec::new(),
            section_start: Some(0),
            index: 0,
            entity_start: 0,
            base_state: State::Text,
            in_rc_data: false,
            in_xml: false,
            in_v_pre: false,
            newlines: Vec::new(),
            mode: ParseMode::BASE,
            delimiter_open: vec![123, 123],  // "{{"
            delimiter_close: vec![125, 125], // "}}"
            delimiter_index: None,
            current_sequence: Vec::new(),
            sequence_index: 0,
            context,
            sequences: Sequences::new(),
        }
    }

    pub fn in_sfc_root(&self) -> bool {
        self.mode == ParseMode::SFC && self.context.stack.len() == 0
    }

    /// Generate Position object with line / column information using recorded
    /// newline positions. We know the index is always going to be an already
    /// processed index, so all the newlines up to this index should have been
    /// recorded.
    pub fn get_pos(&self, index: usize) -> Position {
        let mut line = 1;
        let mut column = index + 1;
        for (i, newline_index) in self.newlines.iter().rev().enumerate() {
            if &index > newline_index {
                line = i + 2;
                column = index - newline_index;
                break;
            }
        }
        Position {
            column,
            line,
            offset: index,
        }
    }

    fn peek(&self) -> u32 {
        self.buffer[self.index + 1] as u32
    }

    fn state_text(&mut self, c: u32) {
        if c == CharCodes::Lt {
            if let Some(section_start) = self.section_start {
                if self.index > section_start {
                    self.ontext(section_start, self.index);
                }
            } else {
                unreachable!();
            }

            self.state = State::BeforeTagName;
            self.section_start = Some(self.index);
        } else if !self.context.global_compile_time_constants.__browser__ && c == CharCodes::Amp {
            self.start_entity();
        } else if !self.in_v_pre && c == self.delimiter_open[0] {
            self.state = State::InterpolationOpen;
            self.delimiter_index = Some(0);
            self.state_interpolation_open(c);
        }
    }

    fn state_interpolation_open(&mut self, c: u32) {
        let Some(delimiter_index) = self.delimiter_index else {
            unreachable!();
        };
        if c == self.delimiter_open[delimiter_index] {
            if delimiter_index == self.delimiter_open.len() - 1 {
                let start = self.index + 1 - self.delimiter_open.len();
                let Some(section_start) = self.section_start else {
                    unreachable!();
                };
                if start > section_start {
                    self.ontext(section_start, start);
                }
                self.state = State::Interpolation;
                self.section_start = Some(start);
            } else {
                self.delimiter_index = Some(delimiter_index + 1);
            }
        } else if self.in_rc_data {
            self.state = State::InRCDATA;
            self.state_in_rc_data(c);
        } else {
            self.state = State::Text;
            self.state_text(c);
        }
    }

    fn state_interpolation(&mut self, c: u32) {
        if c == self.delimiter_close[0] {
            self.state = State::InterpolationClose;
            self.delimiter_index = Some(0);
            self.state_interpolation_close(c);
        }
    }

    fn state_interpolation_close(&mut self, c: u32) {
        let Some(delimiter_index) = self.delimiter_index else {
            unreachable!();
        };
        if c == self.delimiter_close[delimiter_index] {
            if delimiter_index == self.delimiter_close.len() - 1 {
                let Some(section_start) = self.section_start else {
                    unreachable!();
                };
                self.oninterpolation(section_start, self.index + 1);
                if self.in_rc_data {
                    self.state = State::InRCDATA;
                } else {
                    self.state = State::Text;
                }
                self.section_start = Some(self.index + 1);
            } else {
                self.delimiter_index = Some(delimiter_index + 1);
            }
        } else {
            self.state = State::Interpolation;
            self.state_interpolation(c);
        }
    }

    fn state_special_start_sequence(&mut self, c: u32) {
        let is_end = self.sequence_index == self.current_sequence.len();
        let is_match = if is_end {
            // If we are at the end of the sequence, make sure the tag name has ended
            is_end_of_tag_section(c)
        } else {
            // Otherwise, do a case-insensitive comparison
            (c | 0x20) == self.current_sequence[self.sequence_index]
        };

        if !is_match {
            self.in_rc_data = false;
        } else if !is_end {
            self.sequence_index += 1;
            return;
        }

        self.sequence_index = 0;
        self.state = State::InTagName;
        self.state_in_tag_name(c);
    }

    /// Look for an end tag. For <title> and <textarea>, also decode entities.
    fn state_in_rc_data(&mut self, c: u32) {
        if self.sequence_index == self.current_sequence.len() {
            if c == CharCodes::Gt || is_whitespace(c) {
                let end_of_text = self.index - self.current_sequence.len();

                let Some(section_start) = self.section_start else {
                    unreachable!();
                };
                if section_start < end_of_text {
                    // Spoof the index so that reported locations match up.
                    let actual_index = self.index;
                    self.index = end_of_text;
                    self.ontext(section_start, end_of_text);
                    self.index = actual_index;
                }

                // Skip over the `</`
                self.section_start = Some(end_of_text + 2);
                self.state_in_closing_tag_name(c);
                self.in_rc_data = false;
                // We are done; skip the rest of the function.
                return;
            }

            self.sequence_index = 0;
        }

        if (c | 0x20) == self.current_sequence[self.sequence_index] {
            self.sequence_index += 1;
        } else if self.sequence_index == 0 {
            if self.current_sequence == self.sequences.title_end
                || (self.current_sequence == self.sequences.textarea_end && !self.in_sfc_root())
            {
                // We have to parse entities in <title> and <textarea> tags.
                if !self.context.global_compile_time_constants.__browser__ && c == CharCodes::Amp {
                    self.start_entity();
                } else if !self.in_v_pre && c == self.delimiter_open[0] {
                    // We also need to handle interpolation
                    self.state = State::InterpolationOpen;
                    self.delimiter_index = Some(0);
                    self.state_interpolation_open(c);
                }
            } else if self.fast_forward_to(CharCodes::Lt as u32) {
                // Outside of <title> and <textarea> tags, we can fast-forward.
                self.sequence_index = 1;
            }
        } else {
            // If we see a `<`, set the sequence index to 1; useful for eg. `<</script>`.
            self.sequence_index = if c == CharCodes::Lt { 1 } else { 0 };
        }
    }

    fn state_cdata_sequence(&mut self, c: u32) {
        if c == self.sequences.cdata[self.sequence_index] {
            self.sequence_index += 1;
            if self.sequence_index == self.sequences.cdata.len() {
                self.state = State::InCommentLike;
                self.current_sequence = self.sequences.cdata_end.clone();
                self.sequence_index = 0;
                self.section_start = Some(self.index + 1);
            }
        } else {
            self.sequence_index = 0;
            self.state = State::InDeclaration;
            // Reconsume the character
            self.state_in_declaration(c);
        }
    }

    /// When we wait for one specific character, we can speed things up
    /// by skipping through the buffer until we find it.
    ///
    /// @returns Whether the character was found.
    fn fast_forward_to(&mut self, c: u32) -> bool {
        loop {
            self.index += 1;
            if self.index >= self.buffer.len() {
                break;
            }

            let cc = self.buffer[self.index];
            if cc as u32 == CharCodes::NewLine {
                self.newlines.push(self.index);
            }
            if cc as u32 == c {
                return true;
            }
        }

        // We increment the index at the end of the `parse` loop,
        // so set it to `buffer.length - 1` here.
        //
        // TODO: Refactor `parse` to increment index before calling states.
        self.index = self.buffer.len() - 1;

        false
    }

    /// Comments and CDATA end with `-->` and `]]>`.
    ///
    /// Their common qualities are:
    /// - Their end sequences have a distinct character they start with.
    /// - That character is then repeated, so we have to check multiple repeats.
    /// - All characters but the start character of the sequence can be skipped.
    fn state_in_comment_like(&mut self, c: u32) {
        if c == self.current_sequence[self.sequence_index] {
            self.sequence_index += 1;
            if self.sequence_index == self.current_sequence.len() {
                let Some(section_start) = self.section_start else {
                    unreachable!()
                };
                if self.current_sequence == self.sequences.cdata_end {
                    self.oncdata(section_start, self.index - 2);
                } else {
                    self.oncomment(section_start, self.index - 2);
                }

                self.sequence_index = 0;
                self.section_start = Some(self.index + 1);
                self.state = State::Text;
            }
        } else if self.sequence_index == 0 {
            // Fast-forward to the first character of the sequence
            if self.fast_forward_to(self.current_sequence[0]) {
                self.sequence_index = 1;
            }
        } else if c != self.current_sequence[self.sequence_index - 1] {
            // Allow long sequences, eg. --->, ]]]>
            self.sequence_index = 0;
        }
    }

    fn start_special(&mut self, sequence: Vec<u32>, offset: usize) {
        self.enter_rc_data(sequence, offset);
        self.state = State::SpecialStartSequence;
    }

    pub fn enter_rc_data(&mut self, sequence: Vec<u32>, offset: usize) {
        self.in_rc_data = true;
        self.current_sequence = sequence;
        self.sequence_index = offset;
    }

    fn state_before_tag_name(&mut self, c: u32) {
        if c == CharCodes::ExclamationMark {
            self.state = State::BeforeDeclaration;
            self.section_start = Some(self.index + 1);
        } else if c == CharCodes::Questionmark {
            self.state = State::InProcessingInstruction;
            self.section_start = Some(self.index + 1);
        } else if is_tag_start_char(c) {
            self.section_start = Some(self.index);
            if self.mode == ParseMode::BASE {
                // no special tags in base mode
                self.state = State::InTagName;
            } else if self.in_sfc_root() {
                // SFC mode + root level
                // - everything except <template> is RAWTEXT
                // - <template> with lang other than html is also RAWTEXT
                self.state = State::InSFCRootTagName;
            } else if !self.in_xml {
                // HTML mode
                // - <script>, <style> RAWTEXT
                // - <title>, <textarea> RCDATA
                /* t */
                if c == 116 {
                    self.state = State::BeforeSpecialT;
                } else {
                    /* s */
                    self.state = if c == 115 {
                        State::BeforeSpecialS
                    } else {
                        State::InTagName
                    };
                }
            } else {
                self.state = State::InTagName;
            }
        } else if c == CharCodes::Slash {
            self.state = State::BeforeClosingTagName;
        } else {
            self.state = State::Text;
            self.state_text(c);
        }
    }

    fn state_in_tag_name(&mut self, c: u32) {
        if is_end_of_tag_section(c) {
            self.handle_tag_name(c);
        }
    }

    fn state_in_sfc_root_tag_name(&mut self, c: u32) {
        if is_end_of_tag_section(c) {
            let Some(section_start) = self.section_start else {
                unreachable!();
            };
            let tag = self.get_slice(section_start, self.index);
            if tag != "template" {
                self.enter_rc_data(to_char_codes(format!("</{tag}")), 0);
            }
            self.handle_tag_name(c);
        }
    }

    fn handle_tag_name(&mut self, c: u32) {
        let Some(section_start) = self.section_start else {
            unreachable!();
        };
        self.onopentagname(section_start, self.index);
        self.section_start = None;
        self.state = State::BeforeAttrName;
        self.state_before_attr_name(c);
    }

    fn state_before_closing_tag_name(&mut self, c: u32) {
        if is_whitespace(c) {
            // Ignore
        } else if c == CharCodes::Gt {
            if self.context.global_compile_time_constants.__dev__
                || !self.context.global_compile_time_constants.__browser__
            {
                self.onerr(ErrorCodes::MissingEndTagName, self.index);
            }
            self.state = State::Text;
            // Ignore
            self.section_start = Some(self.index + 1)
        } else {
            self.state = if is_tag_start_char(c) {
                State::InClosingTagName
            } else {
                State::InSpecialComment
            };
            self.section_start = Some(self.index)
        }
    }

    fn state_in_closing_tag_name(&mut self, c: u32) {
        if c == CharCodes::Gt || is_whitespace(c) {
            let Some(section_start) = self.section_start else {
                unreachable!();
            };
            self.onclosetag(section_start, self.index);
            self.section_start = None;
            self.state = State::AfterClosingTagName;
            self.state_after_closing_tag_name(c);
        }
    }

    fn state_after_closing_tag_name(&mut self, c: u32) {
        // Skip everything until ">"
        if c == CharCodes::Gt {
            self.state = State::Text;
            self.section_start = Some(self.index + 1);
        }
    }

    fn state_before_attr_name(&mut self, c: u32) {
        if c == CharCodes::Gt {
            self.onopentagend(self.index);
            if self.in_rc_data {
                self.state = State::InRCDATA;
            } else {
                self.state = State::Text;
            }
            self.section_start = Some(self.index + 1);
        } else if c == CharCodes::Slash {
            self.state = State::InSelfClosingTag;
            if (self.context.global_compile_time_constants.__dev__
                || !self.context.global_compile_time_constants.__browser__)
                && self.peek() != CharCodes::Gt
            {
                self.onerr(ErrorCodes::UnexpectedSolidusInTag, self.index);
            }
        } else if c == CharCodes::Lt && self.peek() == CharCodes::Slash {
            // special handling for </ appearing in open tag state
            // this is different from standard HTML parsing but makes practical sense
            // especially for parsing intermediate input state in IDEs.
            self.onopentagend(self.index);
            self.state = State::BeforeTagName;
            self.section_start = Some(self.index);
        } else if !is_whitespace(c) {
            if (self.context.global_compile_time_constants.__dev__
                || !self.context.global_compile_time_constants.__browser__)
                && c == CharCodes::Eq
            {
                self.onerr(
                    ErrorCodes::UnexpectedEqualsSignBeforeAttributeName,
                    self.index,
                );
            }
            self.handle_attr_start(c);
        }
    }

    fn handle_attr_start(&mut self, c: u32) {
        if c == CharCodes::LowerV && self.peek() == CharCodes::Dash {
            self.state = State::InDirName;
            self.section_start = Some(self.index);
        } else if c == CharCodes::Dot
            || c == CharCodes::Colon
            || c == CharCodes::At
            || c == CharCodes::Number
        {
            self.ondirname(self.index, self.index + 1);
            self.state = State::InDirArg;
            self.section_start = Some(self.index + 1);
        } else {
            self.state = State::InAttrName;
            self.section_start = Some(self.index);
        }
    }

    fn state_in_self_closing_tag(&mut self, c: u32) {
        if c == CharCodes::Gt {
            self.onselfclosingtag(self.index);
            self.state = State::Text;
            self.section_start = Some(self.index + 1);
            // Reset special state, in case of self-closing special tags
            self.in_rc_data = false;
        } else if !is_whitespace(c) {
            self.state = State::BeforeAttrName;
            self.state_before_attr_name(c);
        }
    }

    fn state_in_attr_name(&mut self, c: u32) {
        if c == CharCodes::Eq || is_end_of_tag_section(c) {
            let Some(section_start) = self.section_start else {
                unreachable!();
            };
            self.onattribname(section_start, self.index);
            self.handle_attr_name_end(c);
        } else if (self.context.global_compile_time_constants.__dev__
            || !self.context.global_compile_time_constants.__browser__)
            && (c == CharCodes::DoubleQuote || c == CharCodes::SingleQuote || c == CharCodes::Lt)
        {
            self.onerr(ErrorCodes::UnexpectedCharacterInAttributeName, self.index);
        }
    }

    fn state_in_dir_name(&mut self, c: u32) {
        let Some(section_start) = self.section_start else {
            unreachable!();
        };
        if c == CharCodes::Eq || is_end_of_tag_section(c) {
            self.ondirname(section_start, self.index);
            self.handle_attr_name_end(c);
        } else if c == CharCodes::Colon {
            self.ondirname(section_start, self.index);
            self.state = State::InDirArg;
            self.section_start = Some(self.index + 1);
        } else if c == CharCodes::Dot {
            self.ondirname(section_start, self.index);
            self.state = State::InDirModifier;
            self.section_start = Some(self.index + 1);
        }
    }

    fn state_in_dir_arg(&mut self, c: u32) {
        if c == CharCodes::Eq || is_end_of_tag_section(c) {
            let Some(section_start) = self.section_start else {
                unreachable!();
            };
            self.ondirarg(section_start, self.index);
            self.handle_attr_name_end(c);
        } else if c == CharCodes::LeftSquare {
            self.state = State::InDirDynamicArg;
        } else if c == CharCodes::Dot {
            let Some(section_start) = self.section_start else {
                unreachable!();
            };
            self.ondirarg(section_start, self.index);
            self.state = State::InDirModifier;
            self.section_start = Some(self.index + 1);
        }
    }

    fn state_in_dynamic_dir_arg(&mut self, c: u32) {
        if c == CharCodes::RightSquare {
            self.state = State::InDirArg;
        } else if c == CharCodes::Eq || is_end_of_tag_section(c) {
            let Some(section_start) = self.section_start else {
                unreachable!()
            };
            self.ondirarg(section_start, self.index + 1);
            self.handle_attr_name_end(c);
            if self.context.global_compile_time_constants.__dev__
                || !self.context.global_compile_time_constants.__browser__
            {
                self.onerr(ErrorCodes::XMissingDynamicDirectiveArgumentEnd, self.index);
            }
        }
    }

    fn state_in_dir_modifier(&mut self, c: u32) {
        if c == CharCodes::Eq || is_end_of_tag_section(c) {
            let Some(section_start) = self.section_start else {
                unreachable!()
            };
            self.ondirmodifier(section_start, self.index);
            self.handle_attr_name_end(c);
        } else if c == CharCodes::Dot {
            let Some(section_start) = self.section_start else {
                unreachable!()
            };
            self.ondirmodifier(section_start, self.index);
            self.section_start = Some(self.index + 1);
        }
    }

    fn handle_attr_name_end(&mut self, c: u32) {
        self.section_start = Some(self.index);
        self.state = State::AfterAttrName;
        self.onattribnameend(self.index);
        self.state_after_attr_name(c);
    }

    fn state_after_attr_name(&mut self, c: u32) {
        if c == CharCodes::Eq {
            self.state = State::BeforeAttrValue;
        } else if c == CharCodes::Slash || c == CharCodes::Gt {
            let Some(section_start) = self.section_start else {
                unreachable!();
            };
            self.onattribend(QuoteType::NoValue, section_start);
            self.section_start = None;
            self.state = State::BeforeAttrName;
            self.state_before_attr_name(c);
        } else if !is_whitespace(c) {
            let Some(section_start) = self.section_start else {
                unreachable!();
            };
            self.onattribend(QuoteType::NoValue, section_start);
            self.handle_attr_start(c);
        }
    }

    fn state_before_attr_value(&mut self, c: u32) {
        if c == CharCodes::DoubleQuote {
            self.state = State::InAttrValueDq;
            self.section_start = Some(self.index + 1);
        } else if c == CharCodes::SingleQuote {
            self.state = State::InAttrValueSq;
            self.section_start = Some(self.index + 1);
        } else if !is_whitespace(c) {
            self.section_start = Some(self.index);
            self.state = State::InAttrValueNq;
            // Reconsume token
            self.state_in_attr_value_no_quotes(c);
        }
    }

    fn handle_in_attr_value(&mut self, c: u32, quote: u32) {
        if c == quote
            || (self.context.global_compile_time_constants.__browser__
                && self.fast_forward_to(quote))
        {
            let Some(section_start) = self.section_start else {
                unreachable!();
            };
            self.onattribdata(section_start, self.index);
            self.section_start = None;
            self.onattribend(
                if quote == CharCodes::DoubleQuote {
                    QuoteType::Double
                } else {
                    QuoteType::Single
                },
                self.index + 1,
            );
            self.state = State::BeforeAttrName;
        } else if !self.context.global_compile_time_constants.__browser__ && c == CharCodes::Amp {
            self.start_entity();
        }
    }

    fn state_in_attr_value_double_quotes(&mut self, c: u32) {
        self.handle_in_attr_value(c, CharCodes::DoubleQuote as u32);
    }

    fn state_in_attr_value_single_quotes(&mut self, c: u32) {
        self.handle_in_attr_value(c, CharCodes::SingleQuote as u32);
    }

    fn state_in_attr_value_no_quotes(&mut self, c: u32) {
        if is_whitespace(c) || c == CharCodes::Gt {
            let Some(section_start) = self.section_start else {
                unreachable!();
            };
            self.onattribdata(section_start, self.index);
            self.section_start = None;
            self.onattribend(QuoteType::Unquoted, self.index);
            self.state = State::BeforeAttrName;
            self.state_before_attr_name(c);
        } else if ((self.context.global_compile_time_constants.__dev__
            || !self.context.global_compile_time_constants.__browser__)
            && c == CharCodes::DoubleQuote)
            || c == CharCodes::SingleQuote
            || c == CharCodes::Lt
            || c == CharCodes::Eq
            || c == CharCodes::GraveAccent
        {
            self.onerr(
                ErrorCodes::UnexpectedCharacterInUnquotedAttributeValue,
                self.index,
            )
        } else if !self.context.global_compile_time_constants.__browser__ && c == CharCodes::Amp {
            self.start_entity();
        }
    }

    fn state_before_declaration(&mut self, c: u32) {
        if c == CharCodes::LeftSquare {
            self.state = State::CDATASequence;
            self.sequence_index = 0;
        } else {
            self.state = if c == CharCodes::Dash {
                State::BeforeComment
            } else {
                State::InDeclaration
            };
        }
    }

    fn state_in_declaration(&mut self, c: u32) {
        if c == CharCodes::Gt || self.fast_forward_to(CharCodes::Gt as u32) {
            // this.cbs.ondeclaration(this.sectionStart, this.index)
            self.state = State::Text;
            self.section_start = Some(self.index + 1);
        }
    }

    fn state_in_processing_instruction(&mut self, c: u32) {
        if c == CharCodes::Gt || self.fast_forward_to(CharCodes::Gt as u32) {
            let Some(section_start) = self.section_start else {
                unreachable!();
            };
            self.onprocessinginstruction(section_start, self.index);
            self.state = State::Text;
            self.section_start = Some(self.index + 1);
        }
    }

    fn state_before_comment(&mut self, c: u32) {
        if c == CharCodes::Dash {
            self.state = State::InCommentLike;
            self.current_sequence = self.sequences.comment_end.clone();
            // Allow short comments (eg. <!-->)
            self.sequence_index = 2;
            self.section_start = Some(self.index + 1);
        } else {
            self.state = State::InDeclaration;
        }
    }

    fn state_in_special_comment(&mut self, c: u32) {
        if c == CharCodes::Gt || self.fast_forward_to(CharCodes::Gt as u32) {
            let Some(section_start) = self.section_start else {
                unreachable!();
            };
            self.oncomment(section_start, self.index);
            self.state = State::Text;
            self.section_start = Some(self.index + 1);
        }
    }

    fn state_before_special_s(&mut self, c: u32) {
        if c == self.sequences.script_end[3] {
            self.start_special(self.sequences.script_end.clone(), 4);
        } else if c == self.sequences.style_end[3] {
            self.start_special(self.sequences.style_end.clone(), 4);
        } else {
            self.state = State::InTagName;
            // Consume the token again
            self.state_in_tag_name(c);
        }
    }

    fn state_before_special_t(&mut self, c: u32) {
        if c == self.sequences.title_end[3] {
            self.start_special(self.sequences.title_end.clone(), 4);
        } else if c == self.sequences.textarea_end[3] {
            self.start_special(self.sequences.textarea_end.clone(), 4);
        } else {
            self.state = State::InTagName;
            // Consume the token again
            self.state_in_tag_name(c);
        }
    }

    fn start_entity(&mut self) {
        if !self.context.global_compile_time_constants.__browser__ {
            self.base_state = self.state.clone();
            self.state = State::InEntity;
            self.entity_start = self.index;
            // this.entityDecoder!.startEntity(
            //   this.baseState === State.Text || this.baseState === State.InRCDATA
            //     ? DecodingMode.Legacy
            //     : DecodingMode.Attribute,
            // )
            todo!()
        }
    }

    fn state_in_entity(&mut self) {
        if !self.context.global_compile_time_constants.__browser__ {
            // let length = this.entityDecoder!.write(this.buffer, this.index)

            // If `length` is positive, we are done with the entity.
            // if length >= 0 {
            //     self.state = self.base_state;

            //     if length == 0 {
            //         self.index = self.entity_start;
            //     }
            // } else {
            //     // Mark buffer as consumed.
            //     self.index = self.buffer.len() - 1;
            // }
            todo!()
        }
    }

    /// Iterates through the buffer, calling the function corresponding to the current state.
    ///
    /// States that are more likely to be hit are higher up, as a performance improvement.
    pub fn parse(&mut self, input: &str) {
        self.buffer = input.chars().collect();

        while self.index < self.buffer.len() {
            let c = self.buffer[self.index] as u32;
            if c == CharCodes::NewLine && self.state != State::InEntity {
                self.newlines.push(self.index);
            }

            match self.state {
                State::Text => {
                    self.state_text(c);
                }
                State::InterpolationOpen => {
                    self.state_interpolation_open(c);
                }
                State::Interpolation => {
                    self.state_interpolation(c);
                }
                State::InterpolationClose => {
                    self.state_interpolation_close(c);
                }
                State::SpecialStartSequence => {
                    self.state_special_start_sequence(c);
                }
                State::InRCDATA => {
                    self.state_in_rc_data(c);
                }
                State::CDATASequence => {
                    self.state_cdata_sequence(c);
                }
                State::InAttrValueDq => {
                    self.state_in_attr_value_double_quotes(c);
                }
                State::InAttrName => {
                    self.state_in_attr_name(c);
                }
                State::InDirName => {
                    self.state_in_dir_name(c);
                }
                State::InDirArg => {
                    self.state_in_dir_arg(c);
                }
                State::InDirDynamicArg => {
                    self.state_in_dynamic_dir_arg(c);
                }
                State::InDirModifier => {
                    self.state_in_dir_modifier(c);
                }
                State::InCommentLike => {
                    self.state_in_comment_like(c);
                }
                State::InSpecialComment => {
                    self.state_in_special_comment(c);
                }
                State::BeforeAttrName => {
                    self.state_before_attr_name(c);
                }
                State::InTagName => {
                    self.state_in_tag_name(c);
                }
                State::InSFCRootTagName => {
                    self.state_in_sfc_root_tag_name(c);
                }
                State::InClosingTagName => {
                    self.state_in_closing_tag_name(c);
                }
                State::BeforeTagName => {
                    self.state_before_tag_name(c);
                }
                State::AfterAttrName => {
                    self.state_after_attr_name(c);
                }
                State::InAttrValueSq => {
                    self.state_in_attr_value_single_quotes(c);
                }
                State::BeforeAttrValue => {
                    self.state_before_attr_value(c);
                }
                State::BeforeClosingTagName => {
                    self.state_before_closing_tag_name(c);
                }
                State::AfterClosingTagName => {
                    self.state_after_closing_tag_name(c);
                }
                State::BeforeSpecialS => {
                    self.state_before_special_s(c);
                }
                State::BeforeSpecialT => {
                    self.state_before_special_t(c);
                }
                State::InAttrValueNq => {
                    self.state_in_attr_value_no_quotes(c);
                }
                State::InSelfClosingTag => {
                    self.state_in_self_closing_tag(c);
                }
                State::InDeclaration => {
                    self.state_in_declaration(c);
                }
                State::BeforeDeclaration => {
                    self.state_before_declaration(c);
                }
                State::BeforeComment => {
                    self.state_before_comment(c);
                }
                State::InProcessingInstruction => {
                    self.state_in_processing_instruction(c);
                }
                State::InEntity => {
                    self.state_in_entity();
                }
            }

            self.index += 1;
        }

        self.cleanup();
        self.finish();
    }

    /// Remove data that has already been consumed from the buffer.
    fn cleanup(&mut self) {
        // If we are inside of text or attributes, emit what we already have.
        if self.section_start != Some(self.index) {
            if self.state == State::Text
                || (self.state == State::InRCDATA && self.sequence_index == 0)
            {
                self.ontext(self.section_start.unwrap(), self.index);
                self.section_start = Some(self.index);
            } else if self.state == State::InAttrValueDq
                || self.state == State::InAttrValueSq
                || self.state == State::InAttrValueNq
            {
                let Some(section_start) = self.section_start else {
                    unreachable!();
                };
                self.onattribdata(section_start, self.index);
                self.section_start = Some(self.index);
            }
        }
    }

    fn finish(&mut self) {
        if !self.context.global_compile_time_constants.__browser__ && self.state == State::InEntity
        {
            //   this.entityDecoder!.end()
            self.state = self.base_state.clone();
            todo!();
        }

        self.handle_trailing_data();

        self.onend();
    }

    fn handle_trailing_data(&mut self) {
        let end_index = self.buffer.len();

        // If there is no remaining data, we are done.
        if self.section_start >= Some(end_index) {
            return;
        }

        match self.state {
            State::InCommentLike => {
                let Some(section_start) = self.section_start else {
                    unreachable!();
                };
                if self.current_sequence == self.sequences.cdata_end {
                    self.oncdata(section_start, end_index);
                } else {
                    self.oncomment(section_start, end_index);
                }
            }
            State::InTagName
            | State::BeforeAttrName
            | State::BeforeAttrValue
            | State::AfterAttrName
            | State::InAttrName
            | State::InDirName
            | State::InDirArg
            | State::InDirDynamicArg
            | State::InDirModifier
            | State::InAttrValueSq
            | State::InAttrValueDq
            | State::InAttrValueNq
            | State::InClosingTagName => {
                /*
                 * If we are currently in an opening or closing tag, us not calling the
                 * respective callback signals that the tag should be ignored.
                 */
            }
            _ => {
                let Some(section_start) = self.section_start else {
                    unreachable!();
                };

                self.ontext(section_start, end_index);
            }
        }
    }
}
