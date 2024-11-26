use std::marker::PhantomData;

use halo2_proofs::{
    circuit::{Layouter, SimpleFloorPlanner, Value},
    dev::MockProver,
    plonk::{Advice, Circuit, Column, ConstraintSystem, Error, Expression, Selector},
    poly::Rotation,
};

use ff::Field;
use halo2_proofs::circuit::AssignedCell;

const STEPS: usize = 10;

struct TestCircuit<F: Field> {
    _ph: PhantomData<F>,
    secret: Value<F>,
    assurance: Value<F>
}

#[derive(Clone, Debug)]
struct TestConfig<F: Field + Clone> {
    _ph: PhantomData<F>,
    q_mul: Selector,
    q_compare: Selector,
    advice: Column<Advice>,
}

impl<F: Field> TestCircuit<F> {
    /// This region occupies 3 rows.
    /// La función mul lo que hace es recibir 2 celdas (más objetos necesarios para la construccion
    /// de la traza) y devolver una tercera celda con el producto de las 2 primeras.
    /// En este lugar es que se hace la multiplicación real de valores.
    fn mul(
        config: &<Self as Circuit<F>>::Config,
        layouter: &mut impl Layouter<F>,
        lhs: AssignedCell<F, F>,
        rhs: AssignedCell<F, F>,
    ) -> Result<AssignedCell<F, F>, Error> {
        layouter.assign_region(
            || "mul",
            |mut region| {
                let v0 = lhs.value().cloned();
                let v1 = rhs.value().cloned();
                let v2 =
                    v0 //
                        .and_then(|v0| v1.and_then(|v1| Value::known(v0 * v1)));

                let w0 = region.assign_advice(
                    || "assign w0", //
                    config.advice,
                    0,
                    || v0,
                )?;

                let w1 = region.assign_advice(
                    || "assign w1", //
                    config.advice,
                    1,
                    || v1,
                )?;

                let w2 = region.assign_advice(
                    || "assign w2", //
                    config.advice,
                    2,
                    || v2,
                )?;

                // Here it's enforced the multiplication constraint
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
            assurance: Value::unknown()
        }
    }

    fn configure(meta: &mut ConstraintSystem<F>) -> Self::Config {
        let q_enable = meta.complex_selector();
        let q_compare = meta.complex_selector();
        let advice = meta.advice_column();

        // define a new gate:
        meta.create_gate("vertical-mul", |meta| {
            let w0 = meta.query_advice(advice, Rotation(0));
            let w1 = meta.query_advice(advice, Rotation(1));
            let w3 = meta.query_advice(advice, Rotation(2));
            let q_enable = meta.query_selector(q_enable);
            vec![q_enable * (w0 * w1 - w3)]
        });

        meta.create_gate("compare", |meta| {
            let expected_result = meta.query_advice(advice, Rotation(0));
            let result = meta.query_advice(advice, Rotation(1));
            let q_compare = meta.query_selector(q_compare);
            vec![q_compare * (result - expected_result)]
        });

        TestConfig {
            _ph: PhantomData,
            q_mul: q_enable,
            q_compare,
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
        let a5 = TestCircuit::<F>::mul(
            &config, //
            &mut layouter,
            a3.clone(),
            a2.clone(),
        )?;


        let _ = layouter.assign_region(
            || "expected result",
            |mut region| {
                let exp_result = region.assign_advice(
                    || "expected_result",
                    config.advice,
                    0,
                    || self.assurance.clone(),
                );
                let copied_result = region.assign_advice(
                    || "copied_result",
                    config.advice,
                    1,
                    || a5.value().cloned(),
                );

                config.q_compare.enable(&mut region, 0)?;
                exp_result
            },
        );

        Ok(())
    }
}

fn main() {
    use halo2_proofs::halo2curves::bn256::Fr;

    let circuit = TestCircuit::<Fr> {
        _ph: PhantomData,
        secret: Value::known(Fr::from(2)),
        assurance: Value::known(Fr::from(32))
    };
    let prover = MockProver::run(8, &circuit, vec![]).unwrap();
    prover.verify().unwrap();
}
