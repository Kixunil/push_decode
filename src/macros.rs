macro_rules! mapped_decoder {
    ($name:ident, $inner:ty, $func:expr) => {
        #[derive(Debug)]
        pub struct $name($inner);

        impl crate::Decoder for $name {
            type Value = <$inner as crate::Decoder>::Value;
            type Error = <$inner as crate::Decoder>::Error;

            fn bytes_received(&mut self, bytes: &[u8]) -> Result<usize, Self::Error> {
                self.0.bytes_received()
            }

            fn end(self) -> Result<Self::Value, Self::Error> {
                self.0.end().map($func)
            }
        }
    }
}
