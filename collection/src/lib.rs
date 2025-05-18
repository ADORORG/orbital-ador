use metashrew_support::index_pointer::KeyValuePointer;
use metashrew_support::compat::to_arraybuffer_layout;

use alkanes_runtime::{
  declare_alkane, message::MessageDispatch, storage::StoragePointer, token::Token,
  runtime::AlkaneResponder
};

use alkanes_support::{
  cellpack::Cellpack, id::AlkaneId,
  parcel::{AlkaneTransfer, AlkaneTransferParcel}, response::CallResponse
};

use serde::{Serialize, Deserialize};
use anyhow::{anyhow, Result};
use std::sync::Arc;
mod svg_generator;
use svg_generator::SvgGenerator;

/// Template ID for orbital NFT
const ORBITAL_INSTANCE_ID: u128 = 0x41a2;

/// Name of the NFT collection
const CONTRACT_NAME: &str = "Ador Alkane";

/// Symbol of the NFT collection
const CONTRACT_SYMBOL: &str = "Adr";

/// Number of NFTs to be premined during contract initialization
/// This value can be set to 0 if no premine is needed
const PREMINE_MINTS: u128 = 10;

/// Defines a single minting stage.
#[derive(Clone, Default, Debug, Serialize, Deserialize)]
struct Stage {
    id: u128,
    price_per_item: u64,
    max_mints_per_address: u32,
    whitelist: Vec<String>,
    max_supply: u128,
    start_block: u64,
    end_block: u64,
    total_minted: u128,
}

#[derive(Default)]
pub struct Collection (());

impl AlkaneResponder for Collection {}

#[derive(MessageDispatch)]
enum CollectionMessage {
  #[opcode(0)]
  Initialize,

  #[opcode(69)]
  AuthMintOrbital { count: u128 },

  #[opcode(77)]
  MintInStage { stage_id: u128 },

  #[opcode(99)]
  #[returns(String)]
  GetName,

  #[opcode(100)]
  #[returns(String)]
  GetSymbol,

  #[opcode(101)]
  #[returns(u128)]
  GetTotalSupply,

  #[opcode(102)]
  #[returns(u128)]
  GetOrbitalCount,

  #[opcode(999)]
  #[returns(String)]
  GetAttributes { index: u128 },

  #[opcode(1000)]
  #[returns(Vec<u8>)]
  GetData { index: u128 },

  #[opcode(1001)]
  #[returns(Vec<u8>)]
  GetInstanceAlkaneId { index: u128 },

  #[opcode(1002)]
  #[returns(String)]
  GetInstanceIdentifier { index: u128 }
}

impl Token for Collection {
  fn name(&self) -> String {
    return String::from(CONTRACT_NAME)
  }

  fn symbol(&self) -> String {
    return String::from(CONTRACT_SYMBOL);
  }
}

impl Collection {
    fn initialize(&self) -> Result<CallResponse> {
        self.observe_initialization()?;
        self.initialize_mint_stages()?;
        let context: alkanes_support::context::Context = self.context()?;
        let mut response: CallResponse = CallResponse::forward(&context.incoming_alkanes);

        if PREMINE_MINTS > 0 {
            // Collection token acts as auth token for contract minting without any limits
            response.alkanes.0.push(AlkaneTransfer {
                id: context.myself.clone(),
                value: 1u128,
            });
        }

        Ok(response)
    }

    fn get_name(&self) -> Result<CallResponse> {
        let context: alkanes_support::context::Context = self.context()?;
        let mut response: CallResponse = CallResponse::forward(&context.incoming_alkanes);

        response.data = self.name().into_bytes();

        Ok(response)
    }

    fn get_symbol(&self) -> Result<CallResponse> {
        let context: alkanes_support::context::Context = self.context()?;
        let mut response: CallResponse = CallResponse::forward(&context.incoming_alkanes);

        response.data = self.symbol().into_bytes();

        Ok(response)
    }

    fn get_total_supply(&self) -> Result<CallResponse> {
        let context: alkanes_support::context::Context = self.context()?;
        let mut response: CallResponse = CallResponse::forward(&context.incoming_alkanes);

        response.data = 1u128.to_le_bytes().to_vec();

        Ok(response)
    }

    fn get_orbital_count(&self) -> Result<CallResponse> {
        let context: alkanes_support::context::Context = self.context()?;
        let mut response: CallResponse = CallResponse::forward(&context.incoming_alkanes);

        response.data = self.instances_count().to_le_bytes().to_vec();

        Ok(response)
    }

    fn get_attributes(&self, index: u128) -> Result<CallResponse> {
        let context: alkanes_support::context::Context = self.context()?;
        let mut response: CallResponse = CallResponse::forward(&context.incoming_alkanes);

        let attributes: String = SvgGenerator::get_attributes(index)?;
        response.data = attributes.into_bytes();
        Ok(response)
    }

    fn get_data(&self, index: u128) -> Result<CallResponse> {
        let context: alkanes_support::context::Context = self.context()?;
        let mut response: CallResponse = CallResponse::forward(&context.incoming_alkanes);

        let svg: String = SvgGenerator::generate_svg(index)?;
        response.data = svg.into_bytes();
        Ok(response)
    }

    fn get_instance_alkane_id(&self, index: u128) -> Result<CallResponse> {
        let context: alkanes_support::context::Context = self.context()?;
        let mut response: CallResponse = CallResponse::forward(&context.incoming_alkanes);

        let instance_id: AlkaneId = self.lookup_instance(index)?;

        let mut bytes: Vec<u8> = Vec::with_capacity(32);
        bytes.extend_from_slice(&instance_id.block.to_le_bytes());
        bytes.extend_from_slice(&instance_id.tx.to_le_bytes());

        response.data = bytes;
        Ok(response)
    }

    fn get_instance_identifier(&self, index: u128) -> Result<CallResponse> {
        let context: alkanes_support::context::Context = self.context()?;
        let mut response: CallResponse = CallResponse::forward(&context.incoming_alkanes);

        let instance_id: AlkaneId = self.lookup_instance(index)?;
        let instance_str: String = format!("{}:{}", instance_id.block, instance_id.tx);
        
        response.data = instance_str.into_bytes();
        Ok(response)
    }
  
    /// Mint from a stage
    fn mint_in_stage(&self, stage_id: u128) -> Result<CallResponse> {
        // @todo - determine the minter address from context instead of receiving it as a parameter
        let mut stages: Vec<Stage> = self.get_mint_stages()?;
        // let mut stage: Stage = self.get_mint_stage(stage_id)?;
        let block_height: u64 = self.height();

        let stage: &mut Stage = stages.iter_mut().find(|s| s.id == stage_id).ok_or_else(|| anyhow!("stage with ID {} not found", stage_id))?;

        if stage.start_block > block_height || stage.end_block < block_height {
            return Err(anyhow!("Stage is not active"));
        }

        if stage.total_minted + 1 > stage.max_supply {
            return Err(anyhow!("Exceeds max supply for this stage"));
        }

        // @todo - Implement payment collection,
        // Add storage for payment that did not receive
        // orbital due to block limit or whitelist

        // Increase total_minted for stage
        stage.total_minted += 1;
        // Update the stage
        self.set_mint_stages(stages)?;
        // Proceed with minting
        self.mint_orbital()

    }

    fn auth_mint_orbital(&self, count: u128) -> Result<CallResponse> {
        let context: alkanes_support::context::Context = self.context()?;
        let mut response: CallResponse = CallResponse::forward(&context.incoming_alkanes);

        // Authorized mints
        self.only_owner()?;

        // Check if PREMINE_MINTS is greater than 0
        if PREMINE_MINTS == 0 {
            return Err(anyhow!("Premine minting is not enabled (PREMINE_MINTS is 0)"));
        }

        // Check if the requested mint count plus current auth mint count doesn't exceed PREMINE_MINTS
        let current_auth_mints: u128 = self.get_auth_mint_count();
        if current_auth_mints + count > PREMINE_MINTS {
            return Err(anyhow!("Requested mint count {} plus current auth mints {} would exceed premine limit of {}", 
                count, current_auth_mints, PREMINE_MINTS));
        }

        let mut minted_orbitals: Vec<AlkaneTransfer> = Vec::new();

        for _ in 0..count {
            minted_orbitals.push(self.create_mint_transfer()?);
        }

        // Update the auth mint count
        self.set_auth_mint_count(current_auth_mints + count);

        response.alkanes.0.extend(minted_orbitals);

        Ok(response)
    }

    fn mint_orbital(&self) -> Result<CallResponse> {
        let context: alkanes_support::context::Context = self.context()?;
        let mut response: CallResponse = CallResponse::forward(&context.incoming_alkanes);
        self.observe_mint_per_block()?;
        response.alkanes.0.push(self.create_mint_transfer()?);

        Ok(response)
    }

    fn create_mint_transfer(&self) -> Result<AlkaneTransfer> {
        let index: u128 = self.instances_count();

        if index >= self.max_mints() {
            return Err(anyhow!("Alkane Pandas have fully minted out"));
        }

        let inputs: Vec<u128> = vec![
            0x0, 
            index, 
            self.encode_string_to_u128(CONTRACT_NAME),
            self.encode_string_to_u128(CONTRACT_SYMBOL)
        ];

        let cellpack: Cellpack = Cellpack {
            target: AlkaneId {
                block: 6,
                tx: ORBITAL_INSTANCE_ID,
            },
            inputs,
        };

        let sequence: u128 = self.sequence();
        let response: CallResponse = self.call(&cellpack, &AlkaneTransferParcel::default(), self.fuel())?;

        let orbital_id: AlkaneId = AlkaneId {
            block: 2,
            tx: sequence,
        };

        self.add_instance(&orbital_id)?;

        if response.alkanes.0.len() < 1 {
            Err(anyhow!("orbital token not returned with factory"))
        } else {
            Ok(response.alkanes.0[0])
        }
    }

    fn observe_mint_per_block(&self) -> Result<()> {
        let height: u64 = self.height();
        let max_mints: u32 = self.max_mint_per_block();

        let hash: Vec<u8> = height.to_le_bytes().to_vec();
        let mut pointer: StoragePointer = self.seen_pointer(&hash);

        let current_count: u32 = if pointer.get().len() == 0 {
            0
        } else {
            pointer.get_value::<u32>()
        };

        if current_count < max_mints {
            pointer.set_value::<u32>(current_count + 1);
            Ok(())
        } else {
            Err(anyhow!(format!(
                "mint limit reached for block {}",
                hex::encode(&hash)
            )))
        }
    }

    fn max_mints(&self) -> u128 {
        let stages: Vec<Stage> = self.get_mint_stages().unwrap_or_default();     
        stages.iter().map(|s| s.max_supply).sum()
    }

    fn max_mint_per_block(&self) -> u32 {
        10
    }
    
    fn seen_pointer(&self, hash: &Vec<u8>) -> StoragePointer {
        StoragePointer::from_keyword("/seen/").select(&hash)
    }
    /// Instance pointer
    fn instances_pointer(&self) -> StoragePointer {
        StoragePointer::from_keyword("/instances")
    }
    
    fn instances_count(&self) -> u128 {
        self.instances_pointer().get_value::<u128>()
    }

    fn set_instances_count(&self, count: u128) {
        self.instances_pointer().set_value::<u128>(count);
    }

    fn add_instance(&self, instance_id: &AlkaneId) -> Result<u128> {
        let count: u128 = self.instances_count();
        let new_count: u128 = count.checked_add(1)
        .ok_or_else(|| anyhow!("instances count overflow"))?;

        let mut bytes: Vec<u8> = Vec::with_capacity(32);
        bytes.extend_from_slice(&instance_id.block.to_le_bytes());
        bytes.extend_from_slice(&instance_id.tx.to_le_bytes());

        let bytes_vec: Vec<u8> = new_count.to_le_bytes().to_vec();
        let mut instance_pointer: StoragePointer = self.instances_pointer().select(&bytes_vec);
        instance_pointer.set(Arc::new(bytes));
        
        self.set_instances_count(new_count);
        
        Ok(new_count)
    }
        
    fn lookup_instance(&self, index: u128) -> Result<AlkaneId> {
        // Add 1 to index since instances are stored at 1-based indices
        let storage_index: u128 = index + 1;
        let bytes_vec: Vec<u8> = storage_index.to_le_bytes().to_vec();
        
        let instance_pointer: StoragePointer = self.instances_pointer().select(&bytes_vec);
        
        let bytes: Arc<Vec<u8>> = instance_pointer.get();
        if bytes.len() != 32 {
            return Err(anyhow!("Invalid instance data length"));
        }

        let block_bytes: &[u8] = &bytes[..16];
        let tx_bytes: &[u8] = &bytes[16..];

        let block: u128 = u128::from_le_bytes(block_bytes.try_into().unwrap());
        let tx: u128 = u128::from_le_bytes(tx_bytes.try_into().unwrap());

        Ok(AlkaneId { block, tx })
    }

    /// Get storage pointer for authorized mint count
    fn get_auth_mint_count_pointer(&self) -> StoragePointer {
        StoragePointer::from_keyword("/auth_mint_count")
    }
    /// Get authorized mint count
    fn get_auth_mint_count(&self) -> u128 {
        self.get_auth_mint_count_pointer().get_value()
    }
    /// Set authorized mint count
    fn set_auth_mint_count(&self, count: u128) {
        self.get_auth_mint_count_pointer().set_value(count);
    }
    /// Storage pointer for stages
    fn mint_stages_pointer(&self) -> StoragePointer {
        StoragePointer::from_keyword("/stages")
    }
    /// Set stages (serialized)
    fn set_mint_stages(&self, stages: Vec<Stage>) -> Result<()> {
        let mut stages_pointer: StoragePointer = self.mint_stages_pointer();
        let serialized_stages: Vec<u8> = bincode::serialize(&stages)
            .map_err(|_| anyhow!("Failed to serialize stages"))?;
        
        stages_pointer.set(Arc::new(serialized_stages));
        Ok(())
    }
    /// Get all stages (deserialized)
    fn get_mint_stages(&self) -> Result<Vec<Stage>> {
        let stages_pointer: StoragePointer = self.mint_stages_pointer();
        let stored_data: Arc<Vec<u8>> = stages_pointer.get();
        
        if stored_data.is_empty() {
            return Ok(vec![]); // No stages initialized yet
        }

        let stages: Vec<Stage> = bincode::deserialize(&stored_data)
            .map_err(|_| anyhow!("Failed to deserialize stages"))?;
        
        Ok(stages)
    }
    /// Initialize stages if not already set
    fn initialize_mint_stages(&self) -> Result<()> {
        let stages: Vec<Stage> = self.get_mint_stages()?;

        // Only initialize if stages are not yet set
        if stages.is_empty() {
            let initial_stages: Vec<Stage> = vec![
                Stage {
                    id: 1,
                    price_per_item: 100,
                    max_mints_per_address: 5,
                    whitelist: vec![
                        "tb1pxfgth5u8dpvtwzcfkud87n9sfs56ypymc7gv0r2ydvp64clkdxzsmadr3t".to_string(),
                        "tb1qnfvg3mxy46m6d5znqpxpy5fvy7nxw3p83ns7cg".to_string(),
                        "tb1qla5u9e3rz2840rggsjaz54zk8yn48402khann9".to_string(),
                        "tb1p3azhqgk06m3evczr9fxqxsfg62nahrtdgydh7pvh7nqt9t3cy3ys663xnw".to_string()
                    ],
                    max_supply: 100,
                    start_block: 900000,
                    end_block: 905000,
                    total_minted: 0,
                },
                Stage {
                    id: 2,
                    price_per_item: 200,
                    max_mints_per_address: 3,
                    whitelist: vec![],
                    max_supply: 500,
                    start_block: 905001,
                    end_block: 910000,
                    total_minted: 0,
                },
            ];

            self.set_mint_stages(initial_stages)?;
        }

        Ok(())
    }

    fn only_owner(&self) -> Result<()> {
        let context: alkanes_support::context::Context = self.context()?;

        if context.incoming_alkanes.0.len() != 1 {
            return Err(anyhow!(
                "did not authenticate with only the collection token"
            ));
        }

        let transfer: AlkaneTransfer = context.incoming_alkanes.0[0].clone();
        if transfer.id != context.myself.clone() {
            return Err(anyhow!("supplied alkane is not collection token"));
        }

        if transfer.value < 1 {
            return Err(anyhow!(
                "less than 1 unit of collection token supplied to authenticate"
            ));
        }

        Ok(())
    }

    fn encode_string_to_u128(&self, input: &str) -> u128 {
        let mut value: u128 = 0;
        for (i, byte) in input.bytes().enumerate() {
            value |= (byte as u128) << (8 * i);
        }
        value
    }

}

declare_alkane! {
  impl AlkaneResponder for Collection {
    type Message = CollectionMessage;
  }
}
