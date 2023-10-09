use crate::{
    comments::{rewrite_with_comments, write_comments},
    config::items::ItemBraceStyle,
    formatter::*,
    utils::{
        map::byte_span::{ByteSpan, LeafSpans},
        CurlyBrace,
    },
};
use std::fmt::Write;
use sway_ast::{keywords::Token, ItemTrait, ItemTraitItem, Traits};
use sway_types::{
    ast::{Delimiter, PunctKind},
    Spanned,
};

#[cfg(test)]
mod tests;

impl Format for ItemTrait {
    fn format(
        &self,
        formatted_code: &mut FormattedCode,
        formatter: &mut Formatter,
    ) -> Result<(), FormatterError> {
        // Required for comment formatting
        let start_len = formatted_code.len();
        // `pub `
        if let Some(pub_token) = &self.visibility {
            write!(formatted_code, "{} ", pub_token.span().as_str())?;
        }
        // `trait name`
        write!(
            formatted_code,
            "{} {}",
            self.trait_token.span().as_str(),
            self.name.span().as_str()
        )?;
        // `<T>`
        if let Some(generics) = &self.generics {
            generics.format(formatted_code, formatter)?;
        }
        // `: super_trait + super_trait`
        if let Some((colon_token, traits)) = &self.super_traits {
            write!(formatted_code, "{} ", colon_token.ident().as_str())?;
            traits.format(formatted_code, formatter)?;
        }
        // `where`
        if let Some(where_clause) = &self.where_clause_opt {
            writeln!(formatted_code)?;
            where_clause.format(formatted_code, formatter)?;
        } else {
            write!(formatted_code, " ")?;
        }
        Self::open_curly_brace(formatted_code, formatter)?;
        let trait_items = self.trait_items.get();

        if trait_items.is_empty() {
            write_comments(formatted_code, self.trait_items.span().into(), formatter)?;
        } else {
            for item in trait_items {
                for attr in &item.attribute_list {
                    write!(formatted_code, "{}", &formatter.indent_str()?)?;
                    attr.format(formatted_code, formatter)?;
                }
                match &item.value {
                    sway_ast::ItemTraitItem::Fn(fn_signature, _) => {
                        write!(formatted_code, "{}", formatter.indent_str()?)?;
                        fn_signature.format(formatted_code, formatter)?;
                        write!(formatted_code, "{}", PunctKind::Semicolon.as_char())?;
                    }
                    sway_ast::ItemTraitItem::Const(const_decl, _) => {
                        write!(formatted_code, "{}", formatter.indent_str()?)?;
                        const_decl.format(formatted_code, formatter)?;
                    }
                    sway_ast::ItemTraitItem::Type(type_decl, _) => {
                        write!(formatted_code, "{}", formatter.indent_str()?)?;
                        type_decl.format(formatted_code, formatter)?;
                    }
                    ItemTraitItem::Error(_, _) => {
                        return Err(FormatterError::SyntaxError);
                    }
                }
                writeln!(formatted_code)?;
            }
        }

        Self::close_curly_brace(formatted_code, formatter)?;
        if let Some(trait_defs) = &self.trait_defs_opt {
            write!(formatted_code, " ")?;
            Self::open_curly_brace(formatted_code, formatter)?;
            for trait_items in trait_defs.get() {
                write!(formatted_code, "{}", formatter.indent_str()?)?;
                // format `Annotated<ItemFn>`
                trait_items.format(formatted_code, formatter)?;
            }
            writeln!(formatted_code)?;
            Self::close_curly_brace(formatted_code, formatter)?;
        };

        rewrite_with_comments::<ItemTrait>(
            formatter,
            self.span(),
            self.leaf_spans(),
            formatted_code,
            start_len,
        )?;

        Ok(())
    }
}

impl Format for ItemTraitItem {
    fn format(
        &self,
        formatted_code: &mut FormattedCode,
        formatter: &mut Formatter,
    ) -> Result<(), FormatterError> {
        match self {
            ItemTraitItem::Fn(fn_decl, _) => {
                fn_decl.format(formatted_code, formatter)?;
                writeln!(formatted_code, ";")?;
                Ok(())
            }
            ItemTraitItem::Const(const_decl, _) => {
                const_decl.format(formatted_code, formatter)?;
                writeln!(formatted_code)?;
                Ok(())
            }
            ItemTraitItem::Type(type_decl, _) => {
                type_decl.format(formatted_code, formatter)?;
                writeln!(formatted_code)?;
                Ok(())
            }
            ItemTraitItem::Error(_, _) => Err(FormatterError::SyntaxError),
        }
    }
}

impl CurlyBrace for ItemTrait {
    fn open_curly_brace(
        line: &mut FormattedCode,
        formatter: &mut Formatter,
    ) -> Result<(), FormatterError> {
        let brace_style = formatter.config.items.item_brace_style;
        formatter.indent();
        let open_brace = Delimiter::Brace.as_open_char();
        match brace_style {
            ItemBraceStyle::AlwaysNextLine => {
                // Add opening brace to the next line.
                writeln!(line, "\n{open_brace}")?;
            }
            _ => {
                writeln!(line, "{open_brace}")?;
            }
        }

        Ok(())
    }
    fn close_curly_brace(
        line: &mut FormattedCode,
        formatter: &mut Formatter,
    ) -> Result<(), FormatterError> {
        formatter.unindent();
        write!(line, "{}", Delimiter::Brace.as_close_char())?;
        Ok(())
    }
}

impl Format for Traits {
    fn format(
        &self,
        formatted_code: &mut FormattedCode,
        formatter: &mut Formatter,
    ) -> Result<(), FormatterError> {
        // prefix `PathType`
        self.prefix.format(formatted_code, formatter)?;
        // additional `PathType`s
        //
        // ` + PathType`
        for (add_token, path_type) in self.suffixes.iter() {
            write!(formatted_code, " {} ", add_token.span().as_str())?;
            path_type.format(formatted_code, formatter)?;
        }

        Ok(())
    }
}

impl LeafSpans for ItemTrait {
    fn leaf_spans(&self) -> Vec<ByteSpan> {
        let mut collected_spans = Vec::new();
        if let Some(visibility) = &self.visibility {
            collected_spans.push(ByteSpan::from(visibility.span()));
        }
        collected_spans.push(ByteSpan::from(self.trait_token.span()));
        collected_spans.push(ByteSpan::from(self.name.span()));
        if let Some(super_traits) = &self.super_traits {
            collected_spans.append(&mut super_traits.leaf_spans());
        }
        collected_spans.append(&mut self.trait_items.leaf_spans());
        if let Some(trait_defs) = &self.trait_defs_opt {
            collected_spans.append(&mut trait_defs.leaf_spans());
        }
        collected_spans
    }
}

impl LeafSpans for ItemTraitItem {
    fn leaf_spans(&self) -> Vec<ByteSpan> {
        let mut collected_spans = Vec::new();
        match &self {
            ItemTraitItem::Fn(fn_sig, semicolon) => {
                collected_spans.append(&mut fn_sig.leaf_spans());
                collected_spans.extend(semicolon.as_ref().into_iter().flat_map(|x| x.leaf_spans()));
            }
            ItemTraitItem::Const(const_decl, semicolon) => {
                collected_spans.append(&mut const_decl.leaf_spans());
                collected_spans.extend(semicolon.as_ref().into_iter().flat_map(|x| x.leaf_spans()));
            }
            ItemTraitItem::Error(spans, _) => {
                collected_spans.extend(spans.iter().cloned().map(Into::into));
            }
            ItemTraitItem::Type(type_decl, _) => {
                collected_spans.append(&mut type_decl.leaf_spans())
            }
        };
        collected_spans
    }
}

impl LeafSpans for Traits {
    fn leaf_spans(&self) -> Vec<ByteSpan> {
        let mut collected_spans = self.prefix.leaf_spans();
        collected_spans.append(&mut self.suffixes.leaf_spans());
        collected_spans
    }
}
