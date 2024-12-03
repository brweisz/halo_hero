use std::convert::TryInto;
use std::marker::PhantomData;

use halo2_proofs::{
    circuit::{Layouter, SimpleFloorPlanner},
    dev::MockProver,
    plonk::{self, Circuit, ConstraintSystem},
};

use ff::{Field, PrimeField};
use halo2_proofs::circuit::{AssignedCell, Region, Value};
use halo2_proofs::plonk::{Advice, Column, Fixed, Instance};
use halo2_proofs::poly::Rotation;

struct TestCircuit<F: Field> {
    _ph: PhantomData<F>,
    public_inputs: [Value<F>; 3],
    private_inputs: [Value<F>; 1],
}

#[derive(Clone, Debug)]
struct TestConfig<F: Field + Clone> {
    _ph: PhantomData<F>,
    plonk_chip: PlonkChip<F>,
    pi: Column<Instance>,
    a: Column<Advice>,
    b: Column<Advice>,
    c: Column<Advice>,
}

#[derive(Clone, Debug)]
struct PlonkChip<F> {
    _ph: PhantomData<F>,
    ql: Column<Fixed>,
    qr: Column<Fixed>,
    qm: Column<Fixed>,
    qo: Column<Fixed>,
    qc: Column<Fixed>,
}

impl<F: Field> PlonkChip<F> {
    fn new_for_advices(
        meta: &mut ConstraintSystem<F>,
        pi: Column<Instance>,
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
            let pi_ = meta.query_instance(pi, Rotation::cur());
            let a_ = meta.query_advice(a, Rotation::cur());
            let b_ = meta.query_advice(b, Rotation::cur());
            let c_ = meta.query_advice(c, Rotation::cur());

            let ql_ = meta.query_fixed(ql, Rotation::cur());
            let qr_ = meta.query_fixed(qr, Rotation::cur());
            let qm_ = meta.query_fixed(qm, Rotation::cur());
            let qo_ = meta.query_fixed(qo, Rotation::cur());
            let qc_ = meta.query_fixed(qc, Rotation::cur());

            vec![a_.clone() * ql_ + b_.clone() * qr_ + a_ * b_ * qm_ + qo_ * c_ + qc_]
        });

        Self { _ph: PhantomData, ql, qr, qm, qo, qc }
    }

    fn constrain_advice_to_equal_public_input(&self,
                                              config: &TestConfig<F>,
                                              layouter: &mut impl Layouter<F>,
                                              public_input_index: usize,
                                              cell_to_constrain: AssignedCell<F,F>){
        let _ = layouter.constrain_instance(cell_to_constrain.cell(), config.pi, public_input_index);
    }

    fn multiply_cells(
        &self,
        config: &TestConfig<F>,
        layouter: &mut impl Layouter<F>,
        lhs: AssignedCell<F, F>,
        rhs: AssignedCell<F, F>,
    ) -> Option<AssignedCell<F, F>> {
        let mut result_cell = None;
        let _ = layouter.assign_region(
            || "multiplication",
            |mut region| {
                Self::_assign_plonk_regions(&mut region, config, F::ZERO, F::ZERO, F::ONE, -F::ONE, F::ZERO);

                let a = lhs.copy_advice(|| "Copy a", &mut region, config.a, 0)?;
                let b = rhs.copy_advice(|| "Copy b", &mut region, config.b, 0)?;
                let c_value = a.value().cloned() * b.value().cloned();
                let c = region.assign_advice(|| "Result", config.c, 0, || c_value)?;

                result_cell = Some(c);
                Ok(())
            },
        );
        result_cell
    }

    fn add_cells(
        &self,
        config: &TestConfig<F>,
        layouter: &mut impl Layouter<F>,
        lhs: AssignedCell<F, F>,
        rhs: AssignedCell<F, F>,
    ) -> Option<AssignedCell<F, F>> {
        let mut result_cell = None;
        let _ = layouter.assign_region(
            || "addition",
            |mut region| {
                Self::_assign_plonk_regions(&mut region, config, F::ONE, F::ONE, F::ZERO, -F::ONE, F::ZERO);

                let a = lhs.copy_advice(|| "Copy a", &mut region, config.a, 0)?;
                let b = rhs.copy_advice(|| "Copy b", &mut region, config.b, 0)?;
                let c_value = a.value().cloned() + b.value().cloned();
                let c = region.assign_advice(|| "Result", config.c, 0, || c_value)?;

                result_cell = Some(c);
                Ok(())
            },
        );
        result_cell
    }

    fn new_constant_cell(
        &self,
        config: &TestConfig<F>,
        layouter: &mut impl Layouter<F>,
        constant_value: F,
    ) -> Option<AssignedCell<F, F>> {
        let mut result_cell = None;
        let _ = layouter.assign_region(
            || "constant",
            |mut region| {
                Self::_assign_plonk_regions(&mut region, config, F::ZERO, F::ZERO, F::ZERO, -F::ONE, constant_value);

                let c = region.assign_advice(|| "Result", config.c,
                                             0, || Value::known(constant_value))?;
                result_cell = Some(c);
                Ok(())
            },
        );
        result_cell
    }

    fn enforce_cells_to_be_equal(
        &self,
        config: &TestConfig<F>,
        layouter: &mut impl Layouter<F>,
        lhs: AssignedCell<F, F>,
        rhs: AssignedCell<F, F>,
    ) {
        let _ = layouter.assign_region(
            || "addition",
            |mut region| {
                Self::_assign_plonk_regions(&mut region, config, F::ONE, -F::ONE, F::ZERO, F::ZERO, F::ZERO);

                let _a = lhs.copy_advice(|| "Copy a", &mut region, config.a, 0)?;
                let _b = rhs.copy_advice(|| "Copy b", &mut region, config.b, 0)?;

                Ok(())
            },
        );
    }

    fn _assign_plonk_regions(region: &mut Region<F>, config: &TestConfig<F>,
        ql: F, qr: F, qm: F, qo: F, qc: F) {
        let _ql = region.assign_fixed(|| "Ql", config.plonk_chip.ql, 0, || Value::known(ql));
        let _qr = region.assign_fixed(|| "Qr", config.plonk_chip.qr, 0, || Value::known(qr));
        let _qm = region.assign_fixed(|| "Qm", config.plonk_chip.qm, 0, || Value::known(qm));
        let _qo = region.assign_fixed(|| "Qo", config.plonk_chip.qo, 0, || Value::known(qo));
        let _qc = region.assign_fixed(|| "Qc", config.plonk_chip.qc, 0, || Value::known(qc));
    }
}

impl<F: Field + PrimeField> TestCircuit<F> {
    fn unconstrained(
        &self,
        config: &<TestCircuit<F> as Circuit<F>>::Config,
        layouter: &mut impl Layouter<F>,
        value: Value<F>,
    ) -> Result<AssignedCell<F, F>, plonk::Error> {
        layouter.assign_region(
            || "Free variable",
            |mut region| region.assign_advice(|| "Free variable", config.a, 0, || value),
        )
    }

    fn register_inputs(&self,
                       config: &<TestCircuit<F> as Circuit<F>>::Config,
                       layouter: &mut impl Layouter<F>) -> (Vec<AssignedCell<F,F>>, Vec<AssignedCell<F,F>>){
        let mut public_input_cells = vec![];
        let mut private_input_cells = vec![];
        for value in self.public_inputs {
            public_input_cells.push(self.unconstrained(&config, layouter, value).unwrap());
        }
        for value in self.private_inputs {
            private_input_cells.push(self.unconstrained(&config, layouter, value).unwrap());
        }
        (public_input_cells, private_input_cells)
    }
}

impl<F: Field + PrimeField> Circuit<F> for TestCircuit<F> {
    type Config = TestConfig<F>;
    type FloorPlanner = SimpleFloorPlanner;

    fn without_witnesses(&self) -> Self {
        TestCircuit {
            _ph: PhantomData,
            public_inputs: [Value::unknown(); 3],
            private_inputs: [Value::unknown(); 1],
        }
    }

    fn configure(meta: &mut ConstraintSystem<F>) -> Self::Config {
        let a = meta.advice_column();
        let b = meta.advice_column();
        let c = meta.advice_column();
        let pi = meta.instance_column();

        meta.enable_equality(a);
        meta.enable_equality(b);
        meta.enable_equality(c);
        meta.enable_equality(pi);

        let plonk_chip: PlonkChip<F> = PlonkChip::new_for_advices(meta, pi, a, b, c);

        TestConfig { _ph: PhantomData, plonk_chip, pi, a, b, c }
    }

    #[allow(unused_variables)]
    fn synthesize(
        &self,
        mut config: Self::Config,
        mut layouter: impl Layouter<F>,
    ) -> Result<(), plonk::Error> {
        // Aplica para cualquier programa
        let (public_input_cells, private_input_cells) =
            self.register_inputs(&mut config, &mut layouter);

        // Aplica para el programa específico

        // public_inputs = [x,y,expected_result]
        // private_inputs = [z]
        let x = public_input_cells[0].clone();
        let y = public_input_cells[1].clone();
        let expected_result = public_input_cells[2].clone();
        let z = private_input_cells[0].clone();

        // aux1 == x*y
        let aux1 = config.plonk_chip.multiply_cells(&config, &mut layouter, x.clone(), y.clone()).unwrap();
        // aux2 == aux1 + z
        let aux2 = config.plonk_chip.add_cells(&config, &mut layouter, aux1.clone(), z.clone()).unwrap();
        // aux3 == aux1 * aux2
        let aux3 = config.plonk_chip.multiply_cells(&config, &mut layouter, aux1, aux2).unwrap();
        // y == z
        config.plonk_chip.enforce_cells_to_be_equal(&config, &mut layouter, y.clone(), z);

        // aux3 == expected_result
        config.plonk_chip.enforce_cells_to_be_equal(&config, &mut layouter, aux3, expected_result.clone());

        // Enforce public inputs
        for (i, cell) in [x,y,expected_result].into_iter().enumerate() {
            layouter.constrain_instance(cell.cell(), config.pi, i)?;
        }

        Ok(())
    }
}

fn main() {
    use halo2_proofs::halo2curves::bn256::Fr;

    let public_input_values = vec![Fr::from(1),Fr::from(2), Fr::from(8)];
    let private_input_values = vec![Fr::from(2)];
    let circuit = TestCircuit::<Fr> {
        _ph: PhantomData,
        public_inputs: [
            Value::known(public_input_values[0]),
            Value::known(public_input_values[1]),
            Value::known(public_input_values[2]),
        ],
        private_inputs: [Value::known(private_input_values[0])],
    };
    let prover = MockProver::run(8, &circuit, vec![public_input_values]).unwrap();
    prover.verify().unwrap();
}
