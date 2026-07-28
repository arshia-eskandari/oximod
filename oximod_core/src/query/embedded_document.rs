pub trait EmbeddedDocument {
    type Fields;

    #[doc(hidden)]
    fn fields_with_prefix(prefix: &str) -> Self::Fields;
}

impl<T> EmbeddedDocument for Option<T>
where
    T: EmbeddedDocument,
{
    type Fields = T::Fields;

    fn fields_with_prefix(prefix: &str) -> Self::Fields {
        T::fields_with_prefix(prefix)
    }
}
