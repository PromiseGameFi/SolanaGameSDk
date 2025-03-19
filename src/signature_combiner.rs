pub struct KeyShare {
    // ... existing fields ...
    pub transaction: TransactionData,
}

let signature = Signature {
    r: r.into(),
    s: s.into(),
    v: v_eip155
};
let rlp_tx = tx.rlp_signed(&signature); 