#[macro_export]
macro_rules! mapped_decoder {
    ($($(#[$($attr:tt)*])* $vis:vis struct $name:ident($inner:ty) using $value:ty => $func:expr;)*) => {
        $(
            $(#[$($attr)*])*
            $vis struct $name($inner);

            impl $crate::Decoder for $name {
                type Value = $value;
                type Error = <$inner as $crate::Decoder>::Error;

                fn decode_chunk(&mut self, bytes: &mut &[u8]) -> Result<(), Self::Error> {
                    self.0.decode_chunk(bytes)
                }

                fn end(self) -> Result<Self::Value, Self::Error> {
                    self.0.end().map($func)
                }
            }
        )*
    }
}
