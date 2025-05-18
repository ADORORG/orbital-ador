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

use anyhow::Result;
// use protorune_support::balance_sheet::IntoString;
use std::sync::Arc;

#[derive(Default)]
pub struct OrbitalInstance(());

impl AlkaneResponder for OrbitalInstance {}

#[derive(MessageDispatch)]
enum OrbitalInstanceMessage {
  #[opcode(0)]
  Initialize {
    index: u128
  },

  #[opcode(99)]
  #[returns(String)]
  GetName,

  #[opcode(100)]
  #[returns(String)]
  GetSymbol,

  #[opcode(101)]
  #[returns(u128)]
  GetTotalSupply,

  #[opcode(998)]
  #[returns(String)]
  GetCollectionIdentifier,

  #[opcode(999)]
  #[returns(Vec<u8>)]
  GetNftIndex,

  #[opcode(1000)]
  #[returns(Vec<u8>)]
  GetData,

  #[opcode(1001)]
  #[returns(String)]
  GetContentType,

  #[opcode(1002)]
  #[returns(String)]
  GetAttributes,
}

impl Token for OrbitalInstance {
  fn name(&self) -> String {
    let name: String = "ADORs".to_string();
    format!("{} #{}", name, self.index())
  }

  fn symbol(&self) -> String {
    let symbol: String = "ADR".to_string();
    format!("{} #{}", symbol, self.index())
  }
}

impl OrbitalInstance {
  /// Initialize the NFT instance with a given index
  /// Opcode: 0
  fn initialize(&self, index: u128) -> Result<CallResponse> {
    let context = self.context()?;
    let mut response: CallResponse = CallResponse::forward(&context.incoming_alkanes);

    self.observe_initialization()?;

    self.set_collection_alkane_id(&context.caller);
    self.set_index(index);
    // let _ = self.save_name_to_pointer(self.decode_u128_to_string(name));
    // let _ = self.save_symbol_to_pointer(self.decode_u128_to_string(symbol));

    response.alkanes.0.push(AlkaneTransfer {
      id: context.myself.clone(),
      value: 1u128,
    });

    Ok(response)
  }

  /// Get the name of the NFT
  /// Opcode: 99
  fn get_name(&self) -> Result<CallResponse> {
    let context: alkanes_support::context::Context = self.context()?;
    let mut response: CallResponse = CallResponse::forward(&context.incoming_alkanes);

    response.data = (self.name()).into_bytes().to_vec();

    Ok(response)
  }

  /// Get the symbol of the NFT
  /// Opcode: 100
  fn get_symbol(&self) -> Result<CallResponse> {
    let context: alkanes_support::context::Context = self.context()?;
    let mut response: CallResponse = CallResponse::forward(&context.incoming_alkanes);

    response.data = (self.symbol()).into_bytes().to_vec();

    Ok(response)
  }

  /// Get the total supply of the NFT (always 1 for NFTs)
  /// Opcode: 101
  fn get_total_supply(&self) -> Result<CallResponse> {
    let context: alkanes_support::context::Context = self.context()?;
    let mut response: CallResponse = CallResponse::forward(&context.incoming_alkanes);

    response.data = (&1u128.to_le_bytes()).to_vec();

    Ok(response)
  }

  /// Get the collection identifier
  /// Opcode: 998
  fn get_collection_identifier(&self) -> Result<CallResponse> {
    let context: alkanes_support::context::Context = self.context()?;
    let mut response: CallResponse = CallResponse::forward(&context.incoming_alkanes);

    let collection: AlkaneId = self.collection_ref();
    response.data = format!("{}:{}", collection.block, collection.tx).into_bytes();

    Ok(response)
  }

  /// Get the NFT index
  /// Opcode: 999
  fn get_nft_index(&self) -> Result<CallResponse> {
    let context: alkanes_support::context::Context = self.context()?;

    let mut response: CallResponse = CallResponse::forward(&context.incoming_alkanes);
    response.data = self.index().to_le_bytes().to_vec();

    Ok(response)
  }

  /// Get the NFT data
  /// Opcode: 1000
  fn get_data(&self) -> Result<CallResponse> {
    let context: alkanes_support::context::Context = self.context()?;
    let mut response: CallResponse = CallResponse::forward(&context.incoming_alkanes);

    let collection_id: AlkaneId = self.collection_ref();

    let cellpack: Cellpack = Cellpack {
      target: collection_id,
      inputs: vec![1000, self.index()],
    };

    let call_response: CallResponse = self.staticcall(
      &cellpack,
      &AlkaneTransferParcel::default(),
      self.fuel()
    )?;

    response.data = call_response.data;

    Ok(response)
  }

  /// Get the content type of the NFT
  /// Opcode: 1001
  fn get_content_type(&self) -> Result<CallResponse> {
    let context: alkanes_support::context::Context = self.context()?;
    let mut response: CallResponse = CallResponse::forward(&context.incoming_alkanes);

    response.data = String::from("image/svg+xml").into_bytes().to_vec();

    Ok(response)
  }

  /// Get the attributes of the NFT
  /// Opcode: 1002
  fn get_attributes(&self) -> Result<CallResponse> {
    let context: alkanes_support::context::Context = self.context()?;
    let mut response: CallResponse = CallResponse::forward(&context.incoming_alkanes);

    let collection_id: AlkaneId = self.collection_ref();

    let cellpack: Cellpack = Cellpack {
      target: collection_id,
      inputs: vec![1002, self.index()],
    };

    let call_response: CallResponse = self.staticcall(
      &cellpack,
      &AlkaneTransferParcel::default(),
      self.fuel()
    )?;

    response.data = call_response.data;

    Ok(response)
  }

  // Helper functions
  /// Set the collection Alkane ID
  fn set_collection_alkane_id(&self, id: &AlkaneId) {
    let mut bytes: Vec<u8> = Vec::with_capacity(32);
    bytes.extend_from_slice(&id.block.to_le_bytes());
    bytes.extend_from_slice(&id.tx.to_le_bytes());

    self.collection_alkane_id_pointer().set(Arc::new(bytes));
  }

  /// Get the storage pointer for collection Alkane ID
  fn collection_alkane_id_pointer(&self) -> StoragePointer {
    StoragePointer::from_keyword("/collection-alkane-id")
  }

  /// Get the collection reference
  fn collection_ref(&self) -> AlkaneId {
    let data: Arc<Vec<u8>> = self.collection_alkane_id_pointer().get();
    if data.len() == 0 {
      panic!("Collection reference not found");
    }

    let bytes: &Vec<u8> = data.as_ref();
    AlkaneId {
      block: u128::from_le_bytes(bytes[0..16].try_into().unwrap()),
      tx: u128::from_le_bytes(bytes[16..32].try_into().unwrap()),
    }
  }

  /// Get the storage pointer for index
  fn index_pointer(&self) -> StoragePointer {
    StoragePointer::from_keyword("/index")
  }

  /// Get the current index
  fn index(&self) -> u128 {
    self.index_pointer().get_value::<u128>()
  }

  /// Set the index value
  fn set_index(&self, index: u128) {
    self.index_pointer().set_value::<u128>(index);
  }
}

declare_alkane! {
  impl AlkaneResponder for OrbitalInstance {
    type Message = OrbitalInstanceMessage;
  }
}
