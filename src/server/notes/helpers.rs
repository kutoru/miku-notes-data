use sqlx::{postgres::PgArguments, query::QueryAs, Postgres};

use crate::{proto::notes::{sort, Filters, Note, Sort}, types::BindIter};

// creating constants for these strings so that i can have them type-checked
pub struct SortField;
impl SortField {
    pub const CREATED: &'static str = "created";
    pub const LAST_EDITED: &'static str = "last_edited";
    pub const TITLE: &'static str = "title";
}

pub struct SortType;
impl SortType {
    pub const ASC: &'static str = "ASC";
    pub const DESC: &'static str = "DESC";
}

/// finds the first occurence of "()" inside of the `query`,
/// and for the length of the `arr`, pushes Postgres' "$" placeholders into it
pub fn fill_tuple_placeholder<V>(query: &str, arr: &[V], index_offset: usize) -> String {
    let Some(paren_idx) = query.find("()") else {
        return query.to_owned();
    };

    let placeholders_str = (1..=arr.len())
        .map(|i| format!("${}", i + index_offset))
        .collect::<Vec<String>>()
        .join(",");

    let (s, e) = query.split_at(paren_idx + 1);
    let query_str = format!("{s}{placeholders_str}{e}");

    query_str
}

/// builds a new db query for the `Note` type in the `read_notes` route
pub fn build_read_notes_query<'q>(query_str: &'q str, user_id: i32, filters: &'q Filters) -> QueryAs<'q, Postgres, Note, PgArguments> {
    let mut query = sqlx::query_as::<_, Note>(query_str).bind(user_id);

    // filters

    if let Some(filter_tags) = &filters.filter_tags {
        if !filter_tags.tag_ids.is_empty() {
            query = query.bind_iter(&filter_tags.tag_ids);
        }
    }

    if let Some(filter_date) = &filters.filter_date {
        query = query.bind(filter_date.start).bind(filter_date.end + 1);
    }

    if let Some(filter_date_modif) = &filters.filter_date_modif {
        query = query.bind(filter_date_modif.start).bind(filter_date_modif.end + 1);
    }

    if let Some(filter_search) = &filters.filter_search {
        query = query.bind(format!("%{}%", filter_search.query));
    }

    // pagination

    query
}

/// builds a string for the db query for the `Note` type in the `read_notes` route
pub fn build_read_notes_query_str(sort: &Sort, filters: &Filters) -> String {
    let mut query_str: String = "SELECT * FROM notes WHERE user_id = $1".into();
    let mut param_num = 1;

    // filtering

    if let Some(filter_tags) = &filters.filter_tags {
        if !filter_tags.tag_ids.is_empty() {
            query_str += "\nAND id IN (SELECT note_id FROM note_tags WHERE tag_id IN ())";
            query_str = crate::server::notes::fill_tuple_placeholder(&query_str, &filter_tags.tag_ids, param_num);
            param_num += filter_tags.tag_ids.len();
        } else {
            query_str += "\nAND id NOT IN (SELECT note_id FROM note_tags)";
        }
    }

    if filters.filter_date.is_some() {
        query_str += &format!(
            "\nAND EXTRACT(EPOCH FROM created) BETWEEN ${} AND ${}",
            param_num + 1, param_num + 2,
        );
        param_num += 2;
    }

    if filters.filter_date_modif.is_some() {
        query_str += &format!(
            "\nAND EXTRACT(EPOCH FROM last_edited) BETWEEN ${} AND ${}",
            param_num + 1, param_num + 2,
        );
        param_num += 2;
    }

    if filters.filter_search.is_some() {
        query_str += &format!("\nAND title ILIKE ${}", param_num + 1);
    }

    // ordering

    query_str += "\nORDER BY ";

    query_str += match sort.sort_field() {
        sort::Field::Date => SortField::CREATED,
        sort::Field::DateModif => SortField::LAST_EDITED,
        sort::Field::Title => SortField::TITLE,
    };

    query_str += " ";

    query_str += match sort.sort_type() {
        sort::Type::Asc => SortType::ASC,
        sort::Type::Desc => SortType::DESC,
    };

    query_str += ", id DESC;";

    // paginating

    query_str
}
