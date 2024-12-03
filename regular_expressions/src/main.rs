use std::marker::PhantomData;

use halo2_proofs::{
    circuit::{Layouter, SimpleFloorPlanner, Value},
    dev::MockProver,
    plonk::{
        Advice,
        Circuit,
        Column, //
        ConstraintSystem,
        Error,
        Fixed,
        Selector,
        TableColumn,
    },
    poly::Rotation,
};

use ff::{Field, PrimeField};

// ANCHOR: regex
const ST_I: usize = 10;
const ST_A: usize = 1;
const ST_B: usize = 2;
const ST_C: usize = 3;

// start and done states
const ST_START: usize = ST_I;
const ST_DONE: usize = 4;

// end of file marker:
// "dummy padding character"
const EOF: usize = 0xFFFF;

// conversion of the regular expression: a+b+c
const REGEX: [(usize, usize, Option<char>); 8] = [
    (ST_I, ST_A, Some('a')),
    (ST_I, ST_B, Some('b')),
    (ST_A, ST_A, Some('a')),
    (ST_A, ST_B, Some('b')),
    (ST_B, ST_B, Some('b')),
    (ST_B, ST_C, Some('c')),
    (ST_C, ST_DONE, None),
    (ST_DONE, ST_DONE, None),
];
// ANCHOR_END: regex

const MAX_STR_LEN: usize = 20;

struct TestCircuit<F: Field> {
    _ph: PhantomData<F>,
    str: Value<String>,
    sts: Value<Vec<usize>>,
}

#[derive(Clone, Debug)]
struct TestConfig<F: Field + Clone> {
    _ph: PhantomData<F>,
    q_match: Selector,
    q_regex: Selector,  // enable the regex gate
    automata_state: Column<Advice>, // current state of automaton
    current_character: Column<Advice>, // current character
    table_state_current: TableColumn,
    table_state_next: TableColumn,
    table_transition_char: TableColumn,
    fixed_state: Column<Fixed>,
}

impl<F: PrimeField> Circuit<F> for TestCircuit<F> {
    type Config = TestConfig<F>;
    type FloorPlanner = SimpleFloorPlanner;

    fn without_witnesses(&self) -> Self {
        TestCircuit {
            _ph: PhantomData,
            str: Value::unknown(), // the string
            sts: Value::unknown(), // state of the automaton
        }
    }

    // ANCHOR: columns
    fn configure(meta: &mut ConstraintSystem<F>) -> Self::Config {
        let q_regex = meta.complex_selector();
        let q_match = meta.complex_selector();

        let st = meta.advice_column();
        let ch = meta.advice_column();

        let fix_st = meta.fixed_column();

        let tbl_st_cur = meta.lookup_table_column();
        let tbl_st_nxt = meta.lookup_table_column();
        let tbl_ch = meta.lookup_table_column();

        // ANCHOR_END: columns

        // ANCHOR: fix
        meta.create_gate("fix-st", |meta| {
            let current_state = meta.query_advice(st, Rotation::cur());
            let fixed_state_1 = meta.query_fixed(fix_st, Rotation::cur());
            let enabled_fixed_match = meta.query_selector(q_match);
            vec![enabled_fixed_match * (current_state.clone() - fixed_state_1)]
        });
        // ANCHOR_END: fix

        // ANCHOR: lookup
        meta.lookup("transition-st", |meta| {
            let st_cur = meta.query_advice(st, Rotation::cur());
            let st_nxt = meta.query_advice(st, Rotation::next());
            let ch = meta.query_advice(ch, Rotation::cur());
            let en = meta.query_selector(q_regex);
            vec![
                (en.clone() * st_cur, tbl_st_cur),
                (en.clone() * st_nxt, tbl_st_nxt),
                (en.clone() * ch, tbl_ch),
            ]
        });
        // ANCHOR_END: lookup

        TestConfig {
            _ph: PhantomData,
            q_regex,
            automata_state: st,
            current_character: ch,
            table_state_current: tbl_st_cur,
            table_state_next: tbl_st_nxt,
            table_transition_char: tbl_ch,
            fixed_state: fix_st,
            q_match,
        }
    }

    // ANCHOR: assign_table
    fn synthesize(
        &self,
        config: Self::Config, //
        mut layouter: impl Layouter<F>,
    ) -> Result<(), Error> {
        // assign the transition table
        layouter.assign_table(
            || "table",
            |mut table| {
                // convert the numbers to field elements
                let mut transitions: Vec<(F, F, F)> = vec![
                    // (0, 0, 0) is in the table to account for q_regex = 0
                    (F::ZERO, F::ZERO, F::ZERO),
                ];
                for tx in REGEX.iter() {
                    let (st_cur, st_nxt, ch) = tx;
                    transitions.push((
                        F::from(*st_cur as u64),
                        F::from(*st_nxt as u64),
                        ch.map(|c| F::from(c as u64)).unwrap_or(F::from(EOF as u64)),
                    ));
                }

                // assign the table
                for (offset, (st_cur, st_nxt, char)) in transitions //
                    .into_iter()
                    .enumerate()
                {
                    table.assign_cell(
                        || format!("st_cur"),
                        config.table_state_current,
                        offset,
                        || Value::known(st_cur),
                    )?;
                    table.assign_cell(
                        || format!("st_nxt"),
                        config.table_state_next,
                        offset,
                        || Value::known(st_nxt),
                    )?;
                    table.assign_cell(
                        || format!("char"),
                        config.table_transition_char,
                        offset,
                        || Value::known(char),
                    )?;
                }
                Ok(())
            },
        )?;
        // ANCHOR_END: assign_table

        // ANCHOR: region_start
        layouter.assign_region(
            || "regex",
            |mut region| {
                // at offset 0, the state is ST_START
                region.assign_fixed(|| "initial state", config.fixed_state, 0, || Value::known(F::from(ST_START as u64)))?;

                config.q_match.enable(&mut region, 0)?;
                // ANCHOR_END: region_start

                // ANCHOR: region_steps
                // assign each step
                for i in 0..MAX_STR_LEN {
                    // enable the regex automaton
                    config.q_regex.enable(&mut region, i)?;

                    // state
                    region.assign_advice(
                        || "st",
                        config.automata_state,
                        i,
                        || {
                            self.sts.as_ref().map(|s| {
                                F::from(
                                    s.get(i) //
                                        .cloned()
                                        .unwrap_or(ST_DONE)
                                        as u64,
                                )
                            })
                        },
                    )?;

                    // character
                    region.assign_advice(
                        || "ch",
                        config.current_character,
                        i,
                        || {
                            self.str.as_ref().map(|s| {
                                s.chars()
                                    .nth(i)
                                    .map(|c| F::from(c as u64))
                                    .unwrap_or(F::from(EOF as u64))
                            })
                        },
                    )?;
                }
                // ANCHOR_END: region_steps

                // ANCHOR: region_end
                // at offset MAX_STR_LEN, the state is ST_START
                region.assign_advice(
                    || "st",
                    config.automata_state,
                    MAX_STR_LEN,
                    || Value::known(F::from(ST_DONE as u64)),
                )?;
                region.assign_fixed(|| "final state", config.fixed_state, MAX_STR_LEN, || Value::known(F::from(ST_DONE as u64)))?;
                config.q_match.enable(&mut region, MAX_STR_LEN)?;
                Ok(())
            },
        )?;
        // ANCHOR_END: region_end

        Ok(())
    }
}

fn main() {
    use halo2_proofs::halo2curves::bn256::Fr;

    // run the MockProver
    let circuit = TestCircuit::<Fr> {
        _ph: PhantomData,
        // the string to match
        str: Value::known("aaabbbc".to_string()),
        // manually create a trace of the state transitions
        sts: Value::known(vec![
            ST_I, //a
            ST_A, //a
            ST_A, //a
            ST_A, //b
            ST_B, //b
            ST_B, //b
            ST_B, //c
            ST_C,
            // ST_DONE,
        ]),
    };
    let prover = MockProver::run(8, &circuit, vec![]).unwrap();
    prover.verify().unwrap();
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_initial_state_1(){
        use halo2_proofs::halo2curves::bn256::Fr;
        use super::*;

        // run the MockProver
        let circuit = TestCircuit::<Fr> {
            _ph: PhantomData,
            // the string to match
            str: Value::known("aaabbbc".to_string()),
            // manually create a trace of the state transitions
            sts: Value::known(vec![
                ST_I, //a
                ST_A, //a
                ST_A, //a
                ST_A, //b
                ST_B, //b
                ST_B, //b
                ST_B, //c
                ST_C,
            ]),
        };
        let prover = MockProver::run(8, &circuit, vec![]).unwrap();
        prover.verify().unwrap();
    }

    #[test]
    fn test_initial_state_2(){
        use halo2_proofs::halo2curves::bn256::Fr;
        use super::*;

        // run the MockProver
        let circuit = TestCircuit::<Fr> {
            _ph: PhantomData,
            // the string to match
            str: Value::known("bbbc".to_string()),
            // manually create a trace of the state transitions
            sts: Value::known(vec![
                ST_I, //b
                ST_B, //b
                ST_B, //b
                ST_B, //c
                ST_C,
            ]),
        };
        let prover = MockProver::run(8, &circuit, vec![]).unwrap();
        prover.verify().unwrap();
    }
}