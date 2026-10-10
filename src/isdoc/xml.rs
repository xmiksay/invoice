//! A minimal XML writer: elements in document order, escaped text.

pub struct Xml {
    out: String,
    open: Vec<String>,
}

fn escape(s: &str, out: &mut String) {
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            // Control characters are not allowed in XML 1.0.
            c if (c as u32) < 0x20 && !matches!(c, '\t' | '\n' | '\r') => {}
            c => out.push(c),
        }
    }
}

impl Default for Xml {
    fn default() -> Self {
        Self::new()
    }
}

impl Xml {
    pub fn new() -> Self {
        Self {
            out: String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n"),
            open: Vec::new(),
        }
    }

    /// No XML declaration: a piece of a larger document.
    pub fn fragment() -> Self {
        Self {
            out: String::new(),
            open: Vec::new(),
        }
    }

    /// The text so far, elements still open left open (a streamed document
    /// whose end is written later).
    pub fn unclosed(self) -> String {
        self.out
    }

    fn start(&mut self, name: &str, attrs: &[(&str, &str)]) {
        self.out.push('<');
        self.out.push_str(name);
        for (k, v) in attrs {
            self.out.push(' ');
            self.out.push_str(k);
            self.out.push_str("=\"");
            escape(v, &mut self.out);
            self.out.push('"');
        }
        self.out.push('>');
    }

    pub fn open(&mut self, name: &str, attrs: &[(&str, &str)]) -> &mut Self {
        self.start(name, attrs);
        self.open.push(name.to_string());
        self
    }

    pub fn close(&mut self) -> &mut Self {
        if let Some(name) = self.open.pop() {
            self.out.push_str("</");
            self.out.push_str(&name);
            self.out.push('>');
        }
        self
    }

    pub fn leaf_attrs(&mut self, name: &str, attrs: &[(&str, &str)], text: &str) -> &mut Self {
        self.start(name, attrs);
        escape(text, &mut self.out);
        self.out.push_str("</");
        self.out.push_str(name);
        self.out.push('>');
        self
    }

    pub fn leaf(&mut self, name: &str, text: impl AsRef<str>) -> &mut Self {
        self.leaf_attrs(name, &[], text.as_ref())
    }

    /// `leaf` only when `text` is set.
    pub fn opt(&mut self, name: &str, text: Option<impl AsRef<str>>) -> &mut Self {
        if let Some(t) = text {
            self.leaf(name, t);
        }
        self
    }

    /// Closes whatever is still open.
    pub fn finish(mut self) -> String {
        while !self.open.is_empty() {
            self.close();
        }
        self.out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nests_and_escapes() {
        let mut x = Xml::new();
        x.open("a", &[("k", "1\"2")])
            .leaf("b", "x < y & z")
            .opt("c", None::<&str>)
            .leaf_attrs("d", &[("u", "ks")], "")
            .leaf("e", "bell\u{7}");
        let s = x.finish();
        assert!(
            s.ends_with("<a k=\"1&quot;2\"><b>x &lt; y &amp; z</b><d u=\"ks\"></d><e>bell</e></a>")
        );
        assert!(roxmltree::Document::parse(&s).is_ok());
    }

    #[test]
    fn fragment_left_open() {
        let mut x = Xml::fragment();
        x.open("a", &[]).leaf("b", "1");
        assert_eq!(x.unclosed(), "<a><b>1</b>");
    }
}
