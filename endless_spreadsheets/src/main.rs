use std::marker::PhantomData;

use halo2_proofs::{
    circuit::{Layouter, SimpleFloorPlanner, Value},
    dev::MockProver,
    plonk::{Advice, Circuit, Column, ConstraintSystem, Error, Expression, Selector},
    poly::Rotation,
};

use ff::Field;

const STEPS: usize = 5;

struct TestCircuit<F: Field> {
    _ph: PhantomData<F>,
    values: Value<Vec<F>>,
    // When creating a proof you assign the Values in the circuit struct with the witness and run
    // synthesis. Synthesis then assigns the values in the spreadsheet according to the Values in
    // the circuit struct.
}

#[derive(Clone, Debug)]
struct TestConfig<F: Field + Clone> {
    _ph: PhantomData<F>,
    q_enable: Selector,
    advice: Column<Advice>,
}

impl<F: Field> Circuit<F> for TestCircuit<F> {
    type Config = TestConfig<F>;
    type FloorPlanner = SimpleFloorPlanner;

    fn without_witnesses(&self) -> Self {
        TestCircuit {
            _ph: PhantomData,
            values: Value::unknown(),
        }
    }

    /// the goal of "configuration" is to define this spreadsheet and the gates (constraints) that
    /// act on it. The goal of synthesis will be to fill in the spreadsheet.
    fn configure(meta: &mut ConstraintSystem<F>) -> Self::Config {
        let q_enable = meta.complex_selector();
        let advice = meta.advice_column();

        // define a new gate:
        // next = curr + 1 if q_enable is 1
        meta.create_gate("step", |meta| {
            let curr = meta.query_advice(advice, Rotation::cur());
            let next = meta.query_advice(advice, Rotation::next());
            let q_enable = meta.query_selector(q_enable);
            vec![q_enable * (curr - next + Expression::Constant(F::ONE))]
        });

        TestConfig {
            _ph: PhantomData,
            q_enable,
            advice,
        }
    }

    /// Creating regions and assigning cells in them is exactly the job of the synthesize step
    fn synthesize(
        &self,
        config: Self::Config, //
        mut layouter: impl Layouter<F>,
    ) -> Result<(), Error> {
        layouter.assign_region(
            || "steps", // Nombre de la region
            |mut region| {
                // apply the "step" gate STEPS = 5 times
                for i in 0..STEPS {
                    // assign the witness value to the advice column
                    region.assign_advice(
                        || "assign advice",
                        config.advice,
                        i,
                        || self.values.as_ref().map(|values| values[i]),
                    )?;

                    // turn on the gate
                    config.q_enable.enable(&mut region, i)?;
                }

                // assign the final "next" value
                region.assign_advice(
                    || "assign advice",
                    config.advice,
                    STEPS,
                    || self.values.as_ref().map(|values| values[STEPS]),
                )?;

                Ok(())
            },
        )?;
        Ok(())
    }
}

fn main() {
    use halo2_proofs::halo2curves::bn256::Fr;
    let circuit = TestCircuit::<Fr> {
        _ph: PhantomData,
        values: Value::known(vec![
            Fr::from(1),
            Fr::from(2),
            Fr::from(3),
            Fr::from(4),
            Fr::from(5),
            Fr::from(6),
        ]),
    };
    let prover = MockProver::run(8, &circuit, vec![]).unwrap();
    prover.verify().unwrap();
}
