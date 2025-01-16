use std::ops::Deref;

use context::{debug, Context};
use interface::Plugin;
use markdown::{cmark::CowStr, CodeBlockKind, Event, MarkdownEvent, MarkdownFile, Tag};
use plugy::macros::plugin_impl;
use serde::Deserialize;
use types::{Error, File, RawValue, Table};

#[derive(Debug, Deserialize, Default)]
pub struct DataView;

pub enum DisplayType {
    List,
    Tasks,
    Table,
}

#[plugin_impl]
impl Plugin for DataView {
    fn on_load(&self, ctx: &mut Context) -> Result<(), Error> {
        debug!("Loading the DataView plugin");
        ctx.subscribe(&MarkdownEvent::Tag(Tag::CodeBlock(CodeBlockKind::Fenced(
            "/dataview/".to_owned(),
        ))))?;
        Ok(())
    }

    fn process_file(&self, ctx: &mut Context, file: File) -> Result<File, Error> {
        let mut md: MarkdownFile = (&file).try_into()?;

        let events = md.deref().clone();

        for (index, event) in events.iter().enumerate() {
            if let markdown::Event::Start(markdown::cmark::Tag::CodeBlock(
                markdown::cmark::CodeBlockKind::Fenced(tag),
            )) = event
            {
                if tag.contains("dataview") {
                    let next = md.get_mut(index + 1);
                    if let Some(e) = next {
                        if let markdown::Event::Text(ref text) = e {
                            let res = ctx.query_raw(text)?;
                            let txt = table_to_html(&res);
                            *e = Event::Html(CowStr::Boxed(txt.as_str().into()));
                            md.remove(index);
                            md.remove(index + 2);
                        }
                    }
                }
            }
        }
        let file = md.try_into()?;
        Ok(file)
    }
}

/// Converts the `Table` to a Tailwind CSS styled HTML representation.
pub fn table_to_html(table: &Table) -> String {
    let mut html = String::new();

    // Start the table with Tailwind classes
    html.push_str("<table class=\"min-w-full border-collapse border border-gray-300\">");

    // Add table header
    if !table.columns.is_empty() {
        html.push_str("<thead class=\"bg-gray-100\"><tr>");
        for column in &table.columns {
            html.push_str(&format!(
                "<th class=\"border border-gray-300 px-4 py-2 text-left font-medium text-gray-700\">{}</th>",
                column
            ));
        }
        html.push_str("</tr></thead>");
    }

    // Add table rows
    html.push_str("<tbody>");
    for (row_idx, row) in table.rows.iter().enumerate() {
        let row_class = if row_idx % 2 == 0 {
            "bg-white"
        } else {
            "bg-gray-50"
        };
        html.push_str(&format!("<tr class=\"{}\">", row_class));
        for column in &table.columns {
            let value = row.get(column).unwrap_or(&RawValue::Null);
            let cell_value = match value {
                RawValue::Null => "NULL".to_string(),
                RawValue::Integer(i) => i.to_string(),
                RawValue::Real(f) => f.to_string(),
                RawValue::Text(s) => s.to_owned(),
                RawValue::Blob(b) => format!("Blob({} bytes)", b.len()),
            };
            html.push_str(&format!(
                "<td class=\"border border-gray-300 px-4 py-2 text-gray-600\">{}</td>",
                cell_value
            ));
        }
        html.push_str("</tr>");
    }
    html.push_str("</tbody>");

    // End the table
    html.push_str("</table>");

    html
}
