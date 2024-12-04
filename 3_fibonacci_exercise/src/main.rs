use std::marker::PhantomData;

use halo2_proofs::{
    circuit::{Layouter, SimpleFloorPlanner, Value},
    dev::MockProver,
    plonk::{Advice, Circuit, Column, ConstraintSystem, Error, Expression, Selector},
    poly::Rotation,
};

use ff::Field;

const STEPS: usize = 10;

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

        meta.create_gate("fib", |meta| {
            let current_row = meta.query_advice(advice, Rotation(0));
            let next_row = meta.query_advice(advice, Rotation(1));
            let second_next_row = meta.query_advice(advice, Rotation(2));
            let q_enable = meta.query_selector(q_enable);
            vec![q_enable * (second_next_row - next_row - current_row)]
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
                for i in 0..(STEPS-2) {
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

                // assign the final two values
                region.assign_advice(
                    || "assign advice",
                    config.advice,
                    STEPS-2,
                    || self.values.as_ref().map(|values| values[STEPS-2]),
                )?;
                region.assign_advice(
                    || "assign advice",
                    config.advice,
                    STEPS-1,
                    || self.values.as_ref().map(|values| values[STEPS-1]),
                )?;

                Ok(())
            },
        )?;
        Ok(())
    }
}

fn main() {
    use halo2_proofs::halo2curves::bn256::Fr;

    let mut fib: Vec<Fr> = vec![Fr::from(0), Fr::from(1)];
    for i in 1..STEPS {
        let new = fib[i] + fib[i-1];
        fib.push(new);
    }

    let circuit = TestCircuit::<Fr> {
        _ph: PhantomData,
        values: Value::known(fib),
    };
    let prover = MockProver::run(8, &circuit, vec![]).unwrap();
    prover.verify().unwrap();
}
