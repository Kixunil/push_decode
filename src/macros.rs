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

#[macro_export]
macro_rules! delegate {
    (impl$(<$($gen:ident $(: $gen_bounds:path)?),*>)? Encoder for $ty:ty $(where $($where_ty:ty: $($where_bound:path)?),*)? { $field:tt }) => {
        impl$(<$($gen $(: $gen_bounds)?)*>)? $crate::Encoder for $ty $(where $($where_ty: $($where_bound)?),*)? {
            fn encoded_chunk(&self) -> &[u8] {
                $crate::Encoder::encoded_chunk(&self.$field)
            }

            fn next(&mut self) -> bool {
                $crate::Encoder::next(&mut self.$field)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    #[allow(unused)]
    fn compiles() {
        use crate::encoders::{ByteEncoder, BytesEncoder};
        struct Foo(ByteEncoder);

        delegate! {
            impl Encoder for Foo { 0 }
        };

        struct Bar<B: AsRef<[u8]>>(BytesEncoder<B>);

        delegate! {
            impl<B: AsRef<[u8]>> Encoder for Bar<B> { 0 }
        };

        struct Baz<B: AsRef<[u8]>>(BytesEncoder<B>);

        delegate! {
            impl<B> Encoder for Baz<B> where B: AsRef<[u8]> { 0 }
        };

        struct BazSized<B: AsRef<[u8]>>(BytesEncoder<B>);

        delegate! {
            impl<B> Encoder for BazSized<B> where B: AsRef<[u8]>, B: Sized { 0 }
        };
    }
}
