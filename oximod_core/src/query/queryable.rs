use crate::query::builder::Query;

pub trait Queryable: Sized {
    type Fields;

    fn fields() -> Self::Fields;

    fn query() -> Query<Self> {
        Query::new()
    }
}
