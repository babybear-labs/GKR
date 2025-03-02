mod sumcheck {
    use crate::circuit::circuit::FieldElem;

    #[derive(Debug)]
    enum ProtocolError {
        InvalidSum,
        FinalCheckFailed,
    }

    type Polynomial = Vec<FieldElem>; // Coefficients of a univariate polynomial
}

