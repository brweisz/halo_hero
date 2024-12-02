use std::marker::PhantomData;

use halo2_proofs::{
    circuit::{Layouter, SimpleFloorPlanner},
    dev::MockProver,
    plonk::{self, Circuit, ConstraintSystem},
};

use ff::Field;
use halo2_proofs::circuit::{AssignedCell, Value};
use halo2_proofs::plonk::{Advice, Column, Fixed};
use halo2_proofs::poly::Rotation;

struct PlonkChip<F: Field> {
    ql: Column<Fixed>,
    qr: Column<Fixed>,
    qm: Column<Fixed>,
    qo: Column<Fixed>,
    qc: Column<Fixed>,
}

impl<F: Field> PlonkChip<F> {
    fn new_for_advices(
        meta: &mut ConstraintSystem<F>,
        a: Column<Advice>,
        b: Column<Advice>,
        c: Column<Advice>,
    ) -> Self {
        let ql = meta.fixed_column();
        let qr = meta.fixed_column();
        let qm = meta.fixed_column();
        let qo = meta.fixed_column();
        let qc = meta.fixed_column();

        meta.create_gate("Plonk Gate", |meta| {
            let a_ = meta.query_advice(a, Rotation::cur());
            let b_ = meta.query_advice(b, Rotation::cur());
            let c_ = meta.query_advice(c, Rotation::cur());

            let ql_ = meta.query_fixed(ql, Rotation::cur());
            let qr_ = meta.query_fixed(qr, Rotation::cur());
            let qm_ = meta.query_fixed(qm, Rotation::cur());
            let qo_ = meta.query_fixed(qo, Rotation::cur());
            let qc_ = meta.query_fixed(qc, Rotation::cur());

            vec![a_ * ql_ + b_ * qr_ + a_ * b_ * qm_ + qo_ * c_ + qc_]
        });

        Self { ql, qr, qm, qo, qc }
    }

    fn multiply_cells(&mut self,
                      config: &mut TestConfig<F>,
                      layouter: &mut impl Layouter<F>,
                      lhs: AssignedCell<F, F>,
                      rhs: AssignedCell<F, F>) -> Option<AssignedCell<F,F>>{
        let mut result_cell = None;
        let _ = layouter.assign_region(||"multiplication", |mut region| {
            let _ql = region.assign_fixed(||"Ql", config.plonk_chip.ql, 0, || Value::known(F::ZERO))?;
            let _qr = region.assign_fixed(||"Qr", config.plonk_chip.qr, 0, || Value::known(F::ZERO))?;
            let _qm = region.assign_fixed(||"Qm", config.plonk_chip.qm, 0, || Value::known(F::ONE))?;
            let _qo = region.assign_fixed(||"Qo", config.plonk_chip.qo, 0, || Value::known(-F::ONE))?;
            let _qc = region.assign_fixed(||"Qc", config.plonk_chip.qc, 0, || Value::known(F::ZERO))?;

            let a = lhs.copy_advice(||"Copy a", &mut region, config.a, 0)?;
            let b = rhs.copy_advice(||"Copy b", &mut region, config.b, 0)?;
            let c_value = a.value().cloned() * b.value().cloned();
            let c = region.assign_advice(||"Result", config.c, 0, c_value)?;

            result_cell = Some(c);
            Ok(())
        });
        result_cell
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

        TestConfig {
            _ph: PhantomData,
            plonk_chip,
            a,
            b,
            c,
        }
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
