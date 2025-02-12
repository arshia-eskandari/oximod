pub mod feature;
pub mod error;
pub use error::printable::Printable;

#[cfg(test)]
mod tests {
    use crate::feature::test_lib::add::add;
    use crate::feature::client;

    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }
}
