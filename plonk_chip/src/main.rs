use std::marker::PhantomData;

use halo2_proofs::{
    circuit::{Layouter, SimpleFloorPlanner},
    dev::MockProver,
    plonk::{self, Circuit, ConstraintSystem},
};

use ff::Field;
use halo2_proofs::plonk::{Advice, Column, Fixed};
use halo2_proofs::poly::Rotation;

struct PlonkChip<F: Field> {
    // a: Column<Advice>,
    // b: Column<Advice>,
    // c: Column<Advice>,
    //
    ql: Column<Fixed>,
    qr: Column<Fixed>,
    qm: Column<Fixed>,
    qo: Column<Fixed>,
    qc: Column<Fixed>,
}

impl<F> PlonkChip<F> {
    fn new_for_advices(meta: &mut ConstraintSystem<F>,
                       a: Column<Advice>,
                       b: Column<Advice>,
                       c: Column<Advice>) -> Self
    {
        let ql = meta.fixed_column();
        let qr = meta.fixed_column();
        let qm = meta.fixed_column();
        let qo = meta.fixed_column();
        let qc = meta.fixed_column();

        meta.create_gate("Plonk Gate", |meta|{
            let a_ = meta.query_advice(a, Rotation::cur());
            let b_ = meta.query_advice(b, Rotation::cur());
            let c_ = meta.query_advice(c, Rotation::cur());

            let ql_ = meta.query_fixed(ql, Rotation::cur());
            let qr_ = meta.query_fixed(qr, Rotation::cur());
            let qm_ = meta.query_fixed(qm, Rotation::cur());
            let qo_ = meta.query_fixed(qo, Rotation::cur());
            let qc_ = meta.query_fixed(qc, Rotation::cur());

            vec![a_*ql_ + b_*qr_ + a_*b_*qm_ + qo_*c_ + qc_]
        });

        Self {
            ql,
            qr,
            qm,
            qo,
            qc,
        }
    }
}

struct TestCircuit<F: Field> {
    _ph: PhantomData<F>,
}

#[derive(Clone, Debug)]
struct TestConfig<F: Field + Clone> {
    _ph: PhantomData<F>,
    plonk_chip: PlonkChip<F>,
    a: Column<Advice>,
    b: Column<Advice>,
    c: Column<Advice>,
}

impl<F: Field> Circuit<F> for TestCircuit<F> {
    type Config = TestConfig<F>;
    type FloorPlanner = SimpleFloorPlanner;

    fn without_witnesses(&self) -> Self {
        TestCircuit { _ph: PhantomData }
    }

    fn configure(meta: &mut ConstraintSystem<F>) -> Self::Config {

        let a = meta.advice_column();
        let b = meta.advice_column();
        let c = meta.advice_column();

        meta.enable_equality(a);
        meta.enable_equality(b);
        meta.enable_equality(c);

        let plonk_chip = PlonkChip::new_for_advices(meta, a, b, c);

        TestConfig { _ph: PhantomData, plonk_chip, a, b, c }
    }

    #[allow(unused_variables)]
    fn synthesize(
        &self,
        config: Self::Config,
        layouter: impl Layouter<F>,
    ) -> Result<(), plonk::Error> {
        Ok(())
    }
}

fn main() {
    use halo2_proofs::halo2curves::bn256::Fr;
    let circuit = TestCircuit::<Fr> { _ph: PhantomData };
    let prover = MockProver::run(8, &circuit, vec![]).unwrap();
    prover.verify().unwrap();
}