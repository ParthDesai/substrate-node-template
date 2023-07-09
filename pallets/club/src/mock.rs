use crate as pallet_club;
use crate::ClubWeightInfo;
use frame_support::{
	parameter_types,
	traits::{ConstU128, ConstU16, ConstU64, GenesisBuild},
	PalletId,
};
use sp_core::H256;
use sp_runtime::{
	testing::Header,
	traits::{BlakeTwo256, IdentityLookup},
};

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;
type Block = frame_system::mocking::MockBlock<Test>;
type AccountId = u64;

/// An index to a block.
pub type BlockNumber = u64;

/// Balance of an account.
pub type Balance = u128;

// Configure a mock runtime to test the pallet.
frame_support::construct_runtime!(
	pub enum Test where
		Block = Block,
		NodeBlock = Block,
		UncheckedExtrinsic = UncheckedExtrinsic,
	{
		System: frame_system,
		Club: pallet_club,
		Balances: pallet_balances,
	}
);

impl frame_system::Config for Test {
	type BaseCallFilter = frame_support::traits::Everything;
	type BlockWeights = ();
	type BlockLength = ();
	type DbWeight = ();
	type RuntimeOrigin = RuntimeOrigin;
	type RuntimeCall = RuntimeCall;
	type Index = u64;
	type BlockNumber = BlockNumber;
	type Hash = H256;
	type Hashing = BlakeTwo256;
	type AccountId = AccountId;
	type Lookup = IdentityLookup<Self::AccountId>;
	type Header = Header;
	type RuntimeEvent = RuntimeEvent;
	type BlockHashCount = ConstU64<250>;
	type Version = ();
	type PalletInfo = PalletInfo;
	type AccountData = pallet_balances::AccountData<Balance>;
	type OnNewAccount = ();
	type OnKilledAccount = ();
	type SystemWeightInfo = ();
	type SS58Prefix = ConstU16<42>;
	type OnSetCode = ();
	type MaxConsumers = frame_support::traits::ConstU32<16>;
}

impl pallet_balances::Config for Test {
	type Balance = Balance;
	type DustRemoval = ();
	type RuntimeEvent = RuntimeEvent;
	type ExistentialDeposit = ConstU128<1>;
	type AccountStore = System;
	type WeightInfo = ();
	type MaxLocks = ();
	type MaxReserves = ();
	type ReserveIdentifier = [u8; 8];
	type HoldIdentifier = ();
	type FreezeIdentifier = ();
	type MaxHolds = ();
	type MaxFreezes = ();
}

parameter_types! {
	pub const ClubPalletId: PalletId = PalletId(*b"clubtrsy");
	pub const BlocksPerYear: BlockNumber = 10u64;
	pub const ClubCreationFee: Balance = 100;
	pub const MaxNumberOfYears: u8 = 100;
}

impl pallet_club::Config for Test {
	type Currency = Balances;
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = ClubWeightInfo<Test>;
	type PalletId = ClubPalletId;
	type BlocksPerYear = BlocksPerYear;
	type ClubCreationFee = ClubCreationFee;
	type MaxNumberOfYears = MaxNumberOfYears;
}

pub(crate) fn new_test_ext(
	root_account: AccountId,
	club_creation_fee: Balance,
	balances: Vec<(AccountId, Balance)>,
) -> sp_io::TestExternalities {
	let mut storage = frame_system::GenesisConfig::default().build_storage::<Test>().unwrap();

	let balance_config: pallet_balances::GenesisConfig<Test> =
		pallet_balances::GenesisConfig { balances };
	balance_config.assimilate_storage(&mut storage).unwrap();

	let club_config: pallet_club::GenesisConfig<Test> =
		pallet_club::GenesisConfig { root_account: Some(root_account), club_creation_fee };
	club_config.assimilate_storage(&mut storage).unwrap();

	let mut ext: sp_io::TestExternalities = storage.into();
	ext.execute_with(|| System::set_block_number(1));
	ext
}
