use markdown::{
    cmark::{CowStr, Tag, TagEnd},
    Alignment, Event,
};
use types::{RawValue, Table};

pub fn to_events<'a>(table: &Table) -> Vec<Event<'a>> {
    let mut events = Vec::new();

    // Create a vector for column alignments. Here, all columns have no specific alignment.
    let alignment = table
        .columns
        .iter()
        .map(|_| Alignment::None)
        .collect::<Vec<Alignment>>();

    // Start the table
    events.push(Event::Start(Tag::Table(alignment)));

    // Start the table header
    events.push(Event::Start(Tag::TableHead));

    // Start the header row
    events.push(Event::Start(Tag::TableRow));

    // Add each column header cell
    for column in table.columns.iter() {
        events.push(Event::Start(Tag::TableCell));
        events.push(Event::Text(CowStr::Boxed(column.clone().into_boxed_str())));
        events.push(Event::End(TagEnd::TableCell));
    }

    // End the header row and table header
    events.push(Event::End(TagEnd::TableRow));
    events.push(Event::End(TagEnd::TableHead));

    // Iterate over each row in the table
    for row in &table.rows {
        // Start a new table row
        events.push(Event::Start(Tag::TableRow));

        // Add each cell in the row based on the column order
        for column in table.columns.iter() {
            events.push(Event::Start(Tag::TableCell));

            // Retrieve the cell content for the current column, defaulting to an empty string if missing
            if let Some(cell_content) = row.get(column) {
                let cell_value = match cell_content {
                    RawValue::Null => "NULL".to_string(),
                    RawValue::Integer(i) => i.to_string(),
                    RawValue::Real(f) => f.to_string(),
                    RawValue::Text(s) => s.to_owned(),
                    RawValue::Blob(b) => format!("Blob({} bytes)", b.len()),
                };
                events.push(Event::Text(CowStr::Boxed(cell_value.into_boxed_str())));
            } else {
                events.push(Event::Text(CowStr::Borrowed("")));
            }

            events.push(Event::End(TagEnd::TableCell));
        }

        // End the table row
        events.push(Event::End(TagEnd::TableRow));
    }

    // End the table
    events.push(Event::End(TagEnd::Table));

    events
}
