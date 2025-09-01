#[derive(Debug, Clone, PartialEq, Eq, Copy)]
pub enum Tag {
    // 文档结构标签
    Html,
    Head,
    Body,
    Title,
    Meta,
    Link,
    Script,
    Style,

    // 内容标签
    Div,
    P,
    Span,
    H1,
    H2,
    H3,
    H4,
    H5,
    H6,

    // 列表标签
    Ul,
    Ol,
    Li,

    // 表格标签
    Table,
    Thead,
    Tbody,
    Tr,
    Th,
    Td,

    // 表单标签
    Form,
    Input,
    Textarea,
    Select,
    Option,
    Button,
    Label,

    // 链接和媒体标签
    A,
    Img,
    Video,
    Audio,

    // 语义标签
    Header,
    Footer,
    Nav,
    Main,
    Article,
    Section,
    Aside,

    // 其他常用标签
    Br,
    Hr,
    Code,
    Pre,
    Blockquote,
    Em,
    Strong,
    Small,
    Mark,
    Time,
}

impl From<&str> for Tag {
    fn from(value: &str) -> Self {
        match value.to_lowercase().as_str() {
            // 文档结构标签
            "html" => Tag::Html,
            "head" => Tag::Head,
            "body" => Tag::Body,
            "title" => Tag::Title,
            "meta" => Tag::Meta,
            "link" => Tag::Link,
            "script" => Tag::Script,
            "style" => Tag::Style,

            // 内容标签
            "div" => Tag::Div,
            "p" => Tag::P,
            "span" => Tag::Span,
            "h1" => Tag::H1,
            "h2" => Tag::H2,
            "h3" => Tag::H3,
            "h4" => Tag::H4,
            "h5" => Tag::H5,
            "h6" => Tag::H6,

            // 列表标签
            "ul" => Tag::Ul,
            "ol" => Tag::Ol,
            "li" => Tag::Li,

            // 表格标签
            "table" => Tag::Table,
            "thead" => Tag::Thead,
            "tbody" => Tag::Tbody,
            "tr" => Tag::Tr,
            "th" => Tag::Th,
            "td" => Tag::Td,

            // 表单标签
            "form" => Tag::Form,
            "input" => Tag::Input,
            "textarea" => Tag::Textarea,
            "select" => Tag::Select,
            "option" => Tag::Option,
            "button" => Tag::Button,
            "label" => Tag::Label,

            // 链接和媒体标签
            "a" => Tag::A,
            "img" => Tag::Img,
            "video" => Tag::Video,
            "audio" => Tag::Audio,

            // 语义标签
            "header" => Tag::Header,
            "footer" => Tag::Footer,
            "nav" => Tag::Nav,
            "main" => Tag::Main,
            "article" => Tag::Article,
            "section" => Tag::Section,
            "aside" => Tag::Aside,

            // 其他常用标签
            "br" => Tag::Br,
            "hr" => Tag::Hr,
            "code" => Tag::Code,
            "pre" => Tag::Pre,
            "blockquote" => Tag::Blockquote,
            "em" => Tag::Em,
            "strong" => Tag::Strong,
            "small" => Tag::Small,
            "mark" => Tag::Mark,
            "time" => Tag::Time,

            _ => Tag::Div, // 默认使用div标签
        }
    }
}

impl From<Tag> for &str {
    fn from(value: Tag) -> Self {
        match value {
            // 文档结构标签
            Tag::Html => "html",
            Tag::Head => "head",
            Tag::Body => "body",
            Tag::Title => "title",
            Tag::Meta => "meta",
            Tag::Link => "link",
            Tag::Script => "script",
            Tag::Style => "style",

            // 内容标签
            Tag::Div => "div",
            Tag::P => "p",
            Tag::Span => "span",
            Tag::H1 => "h1",
            Tag::H2 => "h2",
            Tag::H3 => "h3",
            Tag::H4 => "h4",
            Tag::H5 => "h5",
            Tag::H6 => "h6",

            // 列表标签
            Tag::Ul => "ul",
            Tag::Ol => "ol",
            Tag::Li => "li",

            // 表格标签
            Tag::Table => "table",
            Tag::Thead => "thead",
            Tag::Tbody => "tbody",
            Tag::Tr => "tr",
            Tag::Th => "th",
            Tag::Td => "td",

            // 表单标签
            Tag::Form => "form",
            Tag::Input => "input",
            Tag::Textarea => "textarea",
            Tag::Select => "select",
            Tag::Option => "option",
            Tag::Button => "button",
            Tag::Label => "label",

            // 链接和媒体标签
            Tag::A => "a",
            Tag::Img => "img",
            Tag::Video => "video",
            Tag::Audio => "audio",

            // 语义标签
            Tag::Header => "header",
            Tag::Footer => "footer",
            Tag::Nav => "nav",
            Tag::Main => "main",
            Tag::Article => "article",
            Tag::Section => "section",
            Tag::Aside => "aside",

            // 其他常用标签
            Tag::Br => "br",
            Tag::Hr => "hr",
            Tag::Code => "code",
            Tag::Pre => "pre",
            Tag::Blockquote => "blockquote",
            Tag::Em => "em",
            Tag::Strong => "strong",
            Tag::Small => "small",
            Tag::Mark => "mark",
            Tag::Time => "time",
        }
    }
}
