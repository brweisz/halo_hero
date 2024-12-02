use std::marker::PhantomData;

use halo2_proofs::{
    circuit::{Layouter, SimpleFloorPlanner},
    dev::MockProver,
    plonk::{self, Circuit, ConstraintSystem},
};

use ff::Field;
use halo2_proofs::circuit::{AssignedCell, Region, Value};
use halo2_proofs::plonk::{Advice, Column, Fixed};
use halo2_proofs::poly::Rotation;

#[derive(Clone, Debug)]
struct PlonkChip{
    ql: Column<Fixed>,
    qr: Column<Fixed>,
    qm: Column<Fixed>,
    qo: Column<Fixed>,
    qc: Column<Fixed>,
}

impl<F: Field> PlonkChip {
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

    fn multiply_cells(&self,
                      config: &TestConfig<F>,
                      layouter: &mut impl Layouter<F>,
                      lhs: AssignedCell<F, F>,
                      rhs: AssignedCell<F, F>) -> Option<AssignedCell<F,F>>{
        let mut result_cell = None;
        let _ = layouter.assign_region(||"multiplication", |mut region| {
            Self::_assign_plonk_regions(&mut region, config, F::ZERO, F::ZERO, F::ONE, -F::ONE, F::ZERO);

            let a = lhs.copy_advice(||"Copy a", &mut region, config.a, 0)?;
            let b = rhs.copy_advice(||"Copy b", &mut region, config.b, 0)?;
            let c_value = a.value().cloned() * b.value().cloned();
            let c = region.assign_advice(||"Result", config.c, 0, c_value)?;

            result_cell = Some(c);
            Ok(())
        });
        result_cell
    }

    fn add_cells(&self,
                  config: &TestConfig<F>,
                  layouter: &mut impl Layouter<F>,
                  lhs: AssignedCell<F, F>,
                  rhs: AssignedCell<F, F>) -> Option<AssignedCell<F,F>>{
        let mut result_cell = None;
        let _ = layouter.assign_region(||"addition", |mut region| {
            Self::_assign_plonk_regions(&mut region, config, F::ONE, F::ONE, F::ZERO, -F::ONE, F::ZERO);

            let a = lhs.copy_advice(||"Copy a", &mut region, config.a, 0)?;
            let b = rhs.copy_advice(||"Copy b", &mut region, config.b, 0)?;
            let c_value = a.value().cloned() + b.value().cloned();
            let c = region.assign_advice(||"Result", config.c, 0, c_value)?;

            result_cell = Some(c);
            Ok(())
        });
        result_cell
    }

    fn new_constant_cell(&self,
                         config: &TestConfig<F>,
                         layouter: &mut impl Layouter<F>,
                         constant_value: Value<F>) -> Option<AssignedCell<F,F>>{
        let mut result_cell = None;
        let _ = layouter.assign_region(||"constant", |mut region| {
            Self::_assign_plonk_regions(&mut region, config, F::ZERO, F::ZERO, F::ZERO, -F::ONE, constant_value);

            let c = region.assign_advice(||"Result", config.c, 0, constant_value)?;
            result_cell = Some(c);
            Ok(())
        });
        result_cell
    }

    fn enforce_cells_to_be_equal(&self,
                                 config: &TestConfig<F>,
                                 layouter: &mut impl Layouter<F>,
                                 lhs: AssignedCell<F, F>,
                                 rhs: AssignedCell<F, F>){
        let _ = layouter.assign_region(||"addition", |mut region| {
            Self::_assign_plonk_regions(&mut region, config, F::ONE, -F::ONE, F::ZERO, F::ZERO, F::ZERO);

            let _a = lhs.copy_advice(||"Copy a", &mut region, config.a, 0)?;
            let _b = rhs.copy_advice(||"Copy b", &mut region, config.b, 0)?;

            Ok(())
        });
    }

    fn _assign_plonk_regions(region: &mut Region<F>, config: &TestConfig<F>, ql: F, qr: F, qm: F, qo: F, qc: F,){
        let _ql = region.assign_fixed(||"Ql", config.plonk_chip.ql, 0, || Value::known(ql));
        let _qr = region.assign_fixed(||"Qr", config.plonk_chip.qr, 0, || Value::known(qr));
        let _qm = region.assign_fixed(||"Qm", config.plonk_chip.qm, 0, || Value::known(qm));
        let _qo = region.assign_fixed(||"Qo", config.plonk_chip.qo, 0, || Value::known(qo));
        let _qc = region.assign_fixed(||"Qc", config.plonk_chip.qc, 0, || Value::known(qc));
    }
}

struct TestCircuit<F: Field> {
    _ph: PhantomData<F>,
    x: Value<F>,
    y: Value<F>,
    z: Value<F>,
}

#[derive(Clone, Debug)]
struct TestConfig<F: Field + Clone> {
    _ph: PhantomData<F>,
    plonk_chip: PlonkChip,
    a: Column<Advice>,
    b: Column<Advice>,
    c: Column<Advice>,
}

impl<F: Field> TestCircuit<F>{
    fn unconstrained(&self,
                  config: &<TestCircuit<F> as Circuit<F>>::Config,
                  layouter: &mut impl Layouter<F>,
                  value: Value<F>) -> Result<AssignedCell<F, F>, plonk::Error>
    {
        layouter.assign_region(||"Free variable", |mut region|{
            region.assign_advice(
                ||"Free variable",
                config.a,
                0,
                value
            )
        })
    }
}

impl<F: Field> Circuit<F> for TestCircuit<F> {
    type Config = TestConfig<F>;
    type FloorPlanner = SimpleFloorPlanner;

    fn without_witnesses(&self) -> Self {
        TestCircuit {
            _ph: PhantomData,
            x: Value::unknown(),
            y: Value::unknown(),
            z: Value::unknown()
        }
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
        mut layouter: impl Layouter<F>,
    ) -> Result<(), plonk::Error> {

        // Set values
        let x = self.unconstrained(&config, &mut layouter, self.x)?;
        let y = self.unconstrained(&config, &mut layouter, self.y)?;
        let z = self.unconstrained(&config, &mut layouter, self.z)?;

        // aux1 == x*y
        let aux1 = config.plonk_chip.multiply_cells(&config, &mut layouter, x, y.clone()).unwrap();
        // aux2 == aux1 + z
        let aux2 =  config.plonk_chip.add_cells(&config, &mut layouter, aux1.clone(), z.clone()).unwrap();
        // aux3 == aux1 * aux2
        let aux3 = config.plonk_chip.multiply_cells(&config, &mut layouter, aux1, aux2).unwrap();
        // y == z
        config.plonk_chip.enforce_cells_to_be_equal(&config, &mut layouter, y, z);
        // aux3 == 8
        let constant_8 = config.plonk_chip.new_constant_cell(&config, &mut layouter, Value::known(F::from(8))).unwrap();
        config.plonk_chip.enforce_cells_to_be_equal(&config, &mut layouter, aux3, constant_8);

        Ok(())
    }
}

fn main() {
    use halo2_proofs::halo2curves::bn256::Fr;

    let circuit = TestCircuit::<Fr> {
        _ph: PhantomData,
        x: Value::known(Fr::ONE),
        y: Value::known(Fr::from(2)),
        z: Value::known(Fr::from(2)),
    };
    let prover = MockProver::run(8, &circuit, vec![]).unwrap();
    prover.verify().unwrap();
}
