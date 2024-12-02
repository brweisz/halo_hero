use std::marker::PhantomData;

use halo2_proofs::{
    circuit::{Layouter, SimpleFloorPlanner, Value},
    dev::MockProver,
    plonk::{Advice, Circuit, Column, ConstraintSystem, Error},
    poly::Rotation,
};

use ff::Field;
use halo2_proofs::circuit::AssignedCell;
use halo2_proofs::plonk::Fixed;

struct TestCircuit<F: Field> {
    _ph: PhantomData<F>,
    secret: Value<F>,
}

#[derive(Clone, Debug)]
struct TestConfig<F: Field + Clone> {
    _ph: PhantomData<F>,
    fixed: Column<Fixed>, // Tipo de columna Fixed, es el caso general del selector q permite solo 0 ó 1
    advice: Column<Advice>,
}

impl<F: Field> TestCircuit<F> {

    fn constrain_cell_to_be_equal_to_fixed(
        &self,
        config: &<Self as Circuit<F>>::Config,
        mut layouter: impl Layouter<F>,
        value: F,
        variable: AssignedCell<F,F>
    ) -> Result<(), Error> {
        layouter.assign_region(
            || "fixed",
            |mut region| {
                let fixed_cell = region.assign_fixed(
                    ||"assign fixed",
                    config.fixed,
                    0,
                    || Value::known(value)
                )?;
                region.constrain_equal(variable.cell(), fixed_cell.cell())?;
                Ok(())
            }
        )
    }

    fn unconstrained(
        &self,
        config: &<Self as Circuit<F>>::Config,
        layouter: &mut impl Layouter<F>,
        value: Value<F>,
    ) -> Result<AssignedCell<F, F>, Error> {
        layouter.assign_region(
            || "free variable",
            |mut region| {
                region.assign_advice(
                    || "assign w0",
                    config.advice,
                    0,
                    || value,
                )
            },
        )
    }
}

impl<F: Field> Circuit<F> for TestCircuit<F> {
    type Config = TestConfig<F>;
    type FloorPlanner = SimpleFloorPlanner;

    fn without_witnesses(&self) -> Self {
        TestCircuit {
            _ph: PhantomData,
            secret: Value::unknown(),
        }
    }

    fn configure(meta: &mut ConstraintSystem<F>) -> Self::Config {
        let q_fixed = meta.complex_selector();
        let fixed = meta.fixed_column();
        let advice = meta.advice_column();
        meta.enable_equality(advice);
        meta.enable_equality(fixed);

        meta.create_gate("equal-constant", |meta| {
            let w = meta.query_advice(advice, Rotation::cur());
            let c = meta.query_fixed(fixed, Rotation::cur());
            let q_fixed = meta.query_selector(q_fixed);
            vec![q_fixed * (w - c)]
        });

        TestConfig {
            _ph: PhantomData,
            fixed,
            advice,
        }
    }

    fn synthesize(
        &self,
        config: Self::Config,
        mut layouter: impl Layouter<F>,
    ) -> Result<(), Error> {
        let cell = self.unconstrained(&config, &mut layouter, self.secret.clone())?;
        let _constant = self.constrain_cell_to_be_equal_to_fixed(&config, layouter, F::ONE, cell);
        Ok(())
    }
}

fn main() {
    use halo2_proofs::halo2curves::bn256::Fr;

    let circuit = TestCircuit::<Fr> {
        _ph: PhantomData,
        secret: Value::known(Fr::from(1)),
    };
    let prover = MockProver::run(8, &circuit, vec![]).unwrap();
    prover.verify().unwrap();
}
