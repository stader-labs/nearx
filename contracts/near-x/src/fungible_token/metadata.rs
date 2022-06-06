use crate::contract::*;
use near_contract_standards::fungible_token::metadata::{
    FungibleTokenMetadata, FungibleTokenMetadataProvider, FT_METADATA_SPEC,
};
use near_sdk::near_bindgen;

#[near_bindgen]
impl FungibleTokenMetadataProvider for NearxPool {
    fn ft_metadata(&self) -> FungibleTokenMetadata {
        FungibleTokenMetadata {
            spec: FT_METADATA_SPEC.to_string(),
            name: "Stader and Near".to_string(),
            symbol: "NEARX".to_string(),
            icon: None,
            reference: Some("https://nearX.app".into()),
            reference_hash: None,
            decimals: 24,
        }
    }
}
