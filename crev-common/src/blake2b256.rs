use blake2::digest::{self, VariableOutput};

#[derive(Debug, Clone)]
pub struct Blake2b256(blake2::VarBlake2b);

impl Default for Blake2b256 {
    fn default() -> Self {
        Blake2b256(blake2::VarBlake2b::new(32).unwrap())
    }
}

impl digest::Update for Blake2b256 {
    fn update(&mut self, data: impl AsRef<[u8]>) {
        self.0.update(data)
    }
}

impl digest::FixedOutput for Blake2b256 {
    type OutputSize = digest::generic_array::typenum::U32;

    fn finalize_into(self, out: &mut digest::generic_array::GenericArray<u8, Self::OutputSize>) {
        self.0.finalize_variable(|slice| {
            assert_eq!(slice.len(), 32);
            out.copy_from_slice(slice)
        });
    }

    fn finalize_into_reset(
        &mut self,
        out: &mut digest::generic_array::GenericArray<u8, Self::OutputSize>,
    ) {
        self.0.finalize_variable_reset(|slice| {
            assert_eq!(slice.len(), 32);
            out.copy_from_slice(slice)
        });
    }
}

impl digest::Reset for Blake2b256 {
    fn reset(&mut self) {
        self.0.reset()
    }
}
