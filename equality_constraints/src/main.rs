use std::marker::PhantomData;

use halo2_proofs::{
    circuit::{Layouter, SimpleFloorPlanner, Value},
    dev::MockProver,
    plonk::{Advice, Circuit, Column, ConstraintSystem, Error, Selector},
    poly::Rotation,
};

use ff::Field;
use halo2_proofs::circuit::AssignedCell;

struct TestCircuit<F: Field> {
    _ph: PhantomData<F>,
    secret: Value<F>,
}

#[derive(Clone, Debug)]
struct TestConfig<F: Field + Clone> {
    _ph: PhantomData<F>,
    q_mul: Selector,
    advice: Column<Advice>,
}

impl<F: Field> TestCircuit<F> {
    fn mul(
        config: &<Self as Circuit<F>>::Config,
        layouter: &mut impl Layouter<F>,
        lhs: AssignedCell<F, F>,
        rhs: AssignedCell<F, F>,
    ) -> Result<AssignedCell<F, F>, Error> {
        layouter.assign_region(
            || "mul",
            |mut region| {


                let w0: AssignedCell<F, F> = lhs.copy_advice(|| "assign w0", &mut region, config.advice, 0)?;
                let w1 = rhs.copy_advice(|| "assign w1", &mut region, config.advice, 1)?;

                let w2 = w0.value().and_then(|w0| w1.value().and_then(|w1| Value::known((*w0) * (*w1))));
                let w2 = region.assign_advice(|| "assign w2", config.advice, 2, || w2)?;
                config.q_mul.enable(&mut region, 0)?;
                Ok(w2)
            },
        )
    }

    /// This region occupies 1 row. Esta es una función auxiliar para crear una región unitaria
    /// (una única fila)
    fn unconstrained(
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
        let q_enable = meta.complex_selector();
        let advice = meta.advice_column();
        meta.enable_equality(advice);

        // define a new gate:
        meta.create_gate("vertical-mul", |meta| {
            let w0 = meta.query_advice(advice, Rotation(0));
            let w1 = meta.query_advice(advice, Rotation(1));
            let w3 = meta.query_advice(advice, Rotation(2));
            let q_enable = meta.query_selector(q_enable);
            vec![q_enable * (w0 * w1 - w3)]
        });

        TestConfig {
            _ph: PhantomData,
            q_mul: q_enable,
            advice,
        }
    }



    fn synthesize(
        &self,
        config: Self::Config, //
        mut layouter: impl Layouter<F>,
    ) -> Result<(), Error> {
        // create a new free variable
        let a = TestCircuit::<F>::unconstrained(
            &config,
            &mut layouter,
            self.secret.clone(),
        )?;

        // do a few multiplications
        let a2 = TestCircuit::<F>::mul(
            &config, //
            &mut layouter,
            a.clone(),
            a.clone(),
        )?;
        let a3 = TestCircuit::<F>::mul(
            &config, //
            &mut layouter,
            a2.clone(),
            a.clone(),
        )?;
        let _a5 = TestCircuit::<F>::mul(
            &config, //
            &mut layouter,
            a3.clone(),
            a2.clone(),
        )?;

        Ok(())
    }
}

fn main() {
    use halo2_proofs::halo2curves::bn256::Fr;

    let circuit = TestCircuit::<Fr> {
        _ph: PhantomData,
        secret: Value::known(Fr::from(3)),
    };
    let prover = MockProver::run(8, &circuit, vec![]).unwrap();
    prover.verify().unwrap();
}
