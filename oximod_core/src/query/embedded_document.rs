pub trait EmbeddedDocument {
    type Fields;

    #[doc(hidden)]
    fn fields_with_prefix(prefix: &str) -> Self::Fields;
}
