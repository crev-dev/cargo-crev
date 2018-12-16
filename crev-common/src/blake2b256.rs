use digest;
use digest::VariableOutput;

#[derive(Debug, Clone)]
pub struct Blake2b256(blake2::VarBlake2b);

impl Default for Blake2b256 {
    fn default() -> Self {
        Blake2b256(blake2::VarBlake2b::new(32).unwrap())
    }
}

impl digest::Input for Blake2b256 {
    fn input<B: AsRef<[u8]>>(&mut self, data: B) {
        self.0.input(data)
    }
}

impl digest::FixedOutput for Blake2b256 {
    type OutputSize = digest::generic_array::typenum::U32;

    fn fixed_result(self) -> digest::generic_array::GenericArray<u8, Self::OutputSize> {
        let mut out = digest::generic_array::GenericArray::default();
        self.0.variable_result(|slice| {
            assert_eq!(slice.len(), 32);
            out.copy_from_slice(slice)
        });

        out
    }
}

impl digest::Reset for Blake2b256 {
    fn reset(&mut self) {
        self.0.reset()
    }
}
