#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::traits::Currency;
pub use pallet::*;
use sp_core::ConstU64;
use sp_runtime::traits::CheckedMul;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;
pub mod weights;
pub use weights::*;

pub(crate) type BalanceOf<T> =
	<<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_support::{
		pallet_prelude::*,
		traits::{ExistenceRequirement, ReservableCurrency},
		PalletId,
	};
	use frame_system::pallet_prelude::*;
	use sp_runtime::{traits::AccountIdConversion, ArithmeticError};

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	/// Configure the pallet by specifying the parameters and types on which it depends.
	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// Currency account
		type Currency: ReservableCurrency<<Self as frame_system::Config>::AccountId>;
		/// Because this pallet emits events, it depends on the runtime's definition of an event.
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
		/// Type representing the weight of this pallet
		type WeightInfo: WeightInfo;
		/// Number of blocks to make one year
		type BlocksPerYear: Get<<Self as frame_system::Config>::BlockNumber>;
		/// ID of the pallet which is used to derive sovereign account id of the pallet
		#[pallet::constant]
		type PalletId: Get<PalletId>;
		/// Club creation fee
		#[pallet::constant]
		type ClubCreationFee: Get<BalanceOf<Self>>;
		/// Max number of years
		#[pallet::constant]
		type MaxNumberOfYears: Get<u8>;
	}

	#[pallet::genesis_config]
	#[derive(frame_support::DefaultNoBound)]
	pub struct GenesisConfig<T: Config> {
		pub root_account: Option<T::AccountId>,
		pub club_creation_fee: BalanceOf<T>,
	}

	#[pallet::genesis_build]
	impl<T: Config> GenesisBuild<T> for GenesisConfig<T> {
		fn build(&self) {
			assert!(self.root_account.is_some(), "Root account must be provided");
			RootAccount::<T>::set(self.root_account.clone());
			assert!(
				self.club_creation_fee >= T::Currency::minimum_balance(),
				"Club creation fee: {:?} must be greater than min balance: {:?}",
				self.club_creation_fee,
				T::Currency::minimum_balance()
			);
			ClubCreationFee::<T>::set(self.club_creation_fee);
		}
	}

	#[derive(Debug, Copy, Clone, Eq, PartialEq, Encode, Decode, MaxEncodedLen, TypeInfo)]
	pub struct ClubDetails<AccountId, Balance> {
		pub owner: AccountId,
		pub expense_per_year: Balance,
	}

	#[derive(Debug, Copy, Clone, Eq, PartialEq, Encode, Decode, MaxEncodedLen, TypeInfo)]
	pub struct MembershipDetails {
		pub is_renewal: bool,
	}

	#[derive(Debug, Copy, Clone, Eq, PartialEq, Encode, Decode, MaxEncodedLen, TypeInfo)]
	pub struct ExpirationDetails {
		pub previous_membership_details: MembershipDetails,
	}

	#[derive(Debug, Copy, Clone, Eq, PartialEq, Encode, Decode, MaxEncodedLen, TypeInfo)]
	pub struct MembershipRequestDetails<Balance> {
		pub amount_paid: Balance,
		pub time_in_year: u8,
		pub is_renewal: bool,
	}

	#[pallet::storage]
	pub(super) type RootAccount<T: Config> = StorageValue<_, T::AccountId>;

	#[pallet::storage]
	pub(super) type ClubCreationFee<T: Config> = StorageValue<_, BalanceOf<T>, ValueQuery>;

	#[pallet::storage]
	pub(super) type NextClubId<T: Config> = StorageValue<_, u64, ValueQuery, ConstU64<1>>;

	// Club_id => club_details
	#[pallet::storage]
	pub(super) type Clubs<T: Config> =
		StorageMap<_, Identity, u64, ClubDetails<T::AccountId, BalanceOf<T>>>;

	// AccountId => club_id => membership details
	#[pallet::storage]
	pub(super) type ClubMembership<T: Config> =
		StorageDoubleMap<_, Identity, T::AccountId, Identity, u64, MembershipDetails>;

	// Block_number => count of future expiration in `ClubMemberFutureExpirations`
	#[pallet::storage]
	pub(super) type ExpirationsPerBlock<T: Config> = StorageMap<_, Identity, T::BlockNumber, u64>;

	// (Block_number, index) => (account, club_id)
	#[pallet::storage]
	pub(super) type ClubMemberFutureExpirations<T: Config> =
		StorageMap<_, Identity, (T::BlockNumber, u64), (T::AccountId, u64)>;

	// To renew double map of account_id -> club_id -> expiration details
	#[pallet::storage]
	pub(super) type ExpiredMemberships<T: Config> =
		StorageDoubleMap<_, Identity, T::AccountId, Identity, u64, ExpirationDetails>;

	// Membership request by user, can be cancelled anytime.
	#[pallet::storage]
	pub(super) type MembershipRequest<T: Config> = StorageDoubleMap<
		_,
		Identity,
		T::AccountId,
		Identity,
		u64,
		MembershipRequestDetails<BalanceOf<T>>,
	>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// A new club is created. [club_id, club_owner]
		ClubCreated { club_id: u64, club_owner: T::AccountId, annual_expense: BalanceOf<T> },
		/// Club owner is transferred. [club_id, new_owner]
		ClubOwnerChanged { club_id: u64, old_owner: T::AccountId, new_owner: T::AccountId },
		/// Annual expense for club membership is set. [club_id, old_expense, new_expense]
		AnnualExpenseSet {
			club_id: u64,
			old_annual_expense: BalanceOf<T>,
			new_annual_expense: BalanceOf<T>,
		},
		/// A membership was requested
		MembershipRequested {
			club_id: u64,
			requester: T::AccountId,
			expense_to_be_charged: BalanceOf<T>,
			time_in_year: u8,
			is_renewal: bool,
		},
		/// A member is added to the club. [club_id, member]
		MemberAdded { club_id: u64, member: T::AccountId, membership_expiry_block: T::BlockNumber },
		/// A member's membership is expired. [club_id, member]
		MembershipExpired { club_id: u64, member: T::AccountId },
		/// A member's membership is renewed
		MembershipRenewed { club_id: u64, member: T::AccountId },
	}

	// Errors inform users that something went wrong.
	#[pallet::error]
	pub enum Error<T> {
		/// No Root account configured at genesis
		NoRootConfiguredAtGenesis,
		/// Not a root account
		UserIsNotRoot,
		/// No club found
		ClubNotFound,
		/// User is not club owner
		NotClubOwner,
		/// User is already club member
		AlreadyMember,
		/// User is expired member
		ExpiredMember,
		/// User has no membership expiration record
		NoMembershipExpirationFound,
		/// Member is not found
		MemberNotFound,
		/// Membership already requested
		MembershipAlreadyRequested,
		/// Membership request not found
		MembershipRequestNotFound,
		/// Membership request for more than max number of years
		MembershipTimeExceeded,
	}

	impl<T: Config> Pallet<T> {
		/// Sovereign account ID of this pallet for receiving tokens.
		fn account_id() -> T::AccountId {
			T::PalletId::get().into_account_truncating()
		}
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_initialize(n: BlockNumberFor<T>) -> Weight {
			// Get count of membership expiration for particular block
			let maybe_expirations = ExpirationsPerBlock::<T>::get(n);
			if maybe_expirations.is_none() {
				return Weight::zero()
			}
			let expirations = maybe_expirations.unwrap();
			ExpirationsPerBlock::<T>::remove(n);

			// Iterate through expiration records and retrieve details
			for i in 1..=expirations {
				let maybe_member_expiration = ClubMemberFutureExpirations::<T>::get((n, i));
				if maybe_member_expiration.is_none() {
					// This expiration was deleted maybe part of extension. Let's continue
					continue
				}
				ClubMemberFutureExpirations::<T>::remove((n, i));
				let (account_id, club_id) = maybe_member_expiration.unwrap();
				let maybe_membership_details = ClubMembership::<T>::get(&account_id, club_id);
				if maybe_membership_details.is_none() {
					continue
				}

				ClubMembership::<T>::remove(&account_id, club_id);
				ExpiredMemberships::<T>::set(
					&account_id,
					club_id,
					Some(ExpirationDetails {
						previous_membership_details: maybe_membership_details.unwrap(),
					}),
				);

				Self::deposit_event(Event::<T>::MembershipExpired { club_id, member: account_id });
			}

			T::WeightInfo::on_initialize(
				expirations
					.try_into()
					.expect("Number of expiration cannot be more than u32::MAX"),
			)
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Create new club, only root can call
		#[pallet::weight(T::WeightInfo::create_club())]
		#[pallet::call_index(1)]
		pub fn create_club(
			origin: OriginFor<T>,
			owner: T::AccountId,
			expense_per_year: BalanceOf<T>,
		) -> DispatchResult {
			let account_id = ensure_signed(origin)?;

			let maybe_root_account = RootAccount::<T>::get();
			if maybe_root_account.is_none() {
				return Err(Error::<T>::NoRootConfiguredAtGenesis.into())
			}
			let root_account = maybe_root_account.unwrap();
			if account_id != root_account {
				return Err(Error::<T>::UserIsNotRoot.into())
			}

			let club = ClubDetails { owner: owner.clone(), expense_per_year };

			let next_id = NextClubId::<T>::get();

			let club_creation_fee = ClubCreationFee::<T>::get();
			T::Currency::transfer(
				&account_id,
				&Self::account_id(),
				club_creation_fee,
				ExistenceRequirement::KeepAlive,
			)?;

			Clubs::<T>::set(next_id, Some(club));
			NextClubId::<T>::set(next_id + 1);

			Self::deposit_event(Event::<T>::ClubCreated {
				club_id: next_id,
				club_owner: owner,
				annual_expense: expense_per_year,
			});
			Ok(())
		}

		/// Transfer club's ownership (only current owner of club can call)
		#[pallet::weight(T::WeightInfo::transfer_club_ownership())]
		#[pallet::call_index(2)]
		pub fn transfer_club_ownership(
			origin: OriginFor<T>,
			club_id: u64,
			new_owner: T::AccountId,
		) -> DispatchResult {
			let account_id = ensure_signed(origin)?;

			let maybe_club = Clubs::<T>::get(club_id);
			if maybe_club.is_none() {
				return Err(Error::<T>::ClubNotFound.into())
			}

			let mut club = maybe_club.unwrap();
			if club.owner != account_id {
				return Err(Error::<T>::NotClubOwner.into())
			}
			club.owner = new_owner.clone();
			Clubs::<T>::set(club_id, Some(club));

			Self::deposit_event(Event::<T>::ClubOwnerChanged {
				club_id,
				old_owner: account_id,
				new_owner,
			});

			Ok(())
		}

		/// Change club's expense per year
		#[pallet::weight(T::WeightInfo::change_club_expense())]
		#[pallet::call_index(3)]
		pub fn change_club_expense(
			origin: OriginFor<T>,
			club_id: u64,
			new_expense_per_year: BalanceOf<T>,
		) -> DispatchResult {
			let account_id = ensure_signed(origin)?;

			let maybe_club = Clubs::<T>::get(club_id);
			if maybe_club.is_none() {
				return Err(Error::<T>::ClubNotFound.into())
			}

			let mut club = maybe_club.unwrap();
			if club.owner != account_id {
				return Err(Error::<T>::NotClubOwner.into())
			}

			let old_annual_expense = club.expense_per_year;
			club.expense_per_year = new_expense_per_year;
			Clubs::<T>::set(club_id, Some(club));

			Self::deposit_event(Event::<T>::AnnualExpenseSet {
				club_id,
				new_annual_expense: new_expense_per_year,
				old_annual_expense,
			});

			Ok(())
		}

		/// Request for new membership
		#[pallet::weight(T::WeightInfo::request_membership())]
		#[pallet::call_index(4)]
		pub fn request_membership(
			origin: OriginFor<T>,
			club_id: u64,
			time_in_year: u8,
		) -> DispatchResult {
			let account_id = ensure_signed(origin)?;
			let maybe_club = Clubs::<T>::get(club_id);
			if maybe_club.is_none() {
				return Err(Error::<T>::ClubNotFound.into())
			}
			let club = maybe_club.unwrap();

			if MembershipRequest::<T>::get(&account_id, club_id).is_some() {
				return Err(Error::<T>::MembershipAlreadyRequested.into())
			}

			if ClubMembership::<T>::get(&account_id, club_id).is_some() {
				return Err(Error::<T>::AlreadyMember.into())
			}

			if ExpiredMemberships::<T>::get(&account_id, club_id).is_some() {
				return Err(Error::<T>::ExpiredMember.into())
			}

			let max_number_of_years = T::MaxNumberOfYears::get();
			if time_in_year > max_number_of_years {
				return Err(Error::<T>::MembershipTimeExceeded.into())
			}

			let time_of_year_as_balance = BalanceOf::<T>::try_from(time_in_year)
				.map_err(|_| DispatchError::Arithmetic(ArithmeticError::Overflow))?;
			let expense_to_be_charged = club
				.expense_per_year
				.checked_mul(&time_of_year_as_balance)
				.ok_or(DispatchError::Arithmetic(ArithmeticError::Overflow))?;
			T::Currency::transfer(
				&account_id,
				&Self::account_id(),
				expense_to_be_charged,
				ExistenceRequirement::KeepAlive,
			)?;
			let request_details = MembershipRequestDetails {
				amount_paid: expense_to_be_charged,
				time_in_year,
				is_renewal: false,
			};
			MembershipRequest::<T>::set(&account_id, club_id, Some(request_details));

			Self::deposit_event(Event::<T>::MembershipRequested {
				club_id,
				requester: account_id,
				expense_to_be_charged,
				time_in_year,
				is_renewal: false,
			});

			Ok(())
		}

		/// Request for membership renewal
		#[pallet::weight(T::WeightInfo::request_membership_renewal())]
		#[pallet::call_index(5)]
		pub fn request_membership_renewal(
			origin: OriginFor<T>,
			club_id: u64,
			time_in_year: u8,
		) -> DispatchResult {
			let account_id = ensure_signed(origin)?;
			let maybe_club = Clubs::<T>::get(club_id);
			if maybe_club.is_none() {
				return Err(Error::<T>::ClubNotFound.into())
			}
			let club = maybe_club.unwrap();

			if MembershipRequest::<T>::get(&account_id, club_id).is_some() {
				return Err(Error::<T>::MembershipAlreadyRequested.into())
			}

			if ClubMembership::<T>::get(&account_id, club_id).is_some() {
				return Err(Error::<T>::AlreadyMember.into())
			}

			let maybe_expired_membership = ExpiredMemberships::<T>::get(&account_id, club_id);
			if maybe_expired_membership.is_none() {
				return Err(Error::<T>::NoMembershipExpirationFound.into())
			}

			let max_number_of_years = T::MaxNumberOfYears::get();
			if time_in_year > max_number_of_years {
				return Err(Error::<T>::MembershipTimeExceeded.into())
			}

			let time_of_year_as_balance = BalanceOf::<T>::try_from(time_in_year)
				.map_err(|_| DispatchError::Arithmetic(ArithmeticError::Overflow))?;
			let expense_to_be_charged = club
				.expense_per_year
				.checked_mul(&time_of_year_as_balance)
				.ok_or(DispatchError::Arithmetic(ArithmeticError::Overflow))?;
			T::Currency::transfer(
				&account_id,
				&Self::account_id(),
				expense_to_be_charged,
				ExistenceRequirement::KeepAlive,
			)?;

			ExpiredMemberships::<T>::remove(&account_id, club_id);
			let request_details = MembershipRequestDetails {
				amount_paid: expense_to_be_charged,
				time_in_year,
				is_renewal: true,
			};
			MembershipRequest::<T>::set(&account_id, club_id, Some(request_details));

			Self::deposit_event(Event::<T>::MembershipRequested {
				club_id,
				requester: account_id,
				expense_to_be_charged,
				time_in_year,
				is_renewal: true,
			});

			Ok(())
		}

		/// Add member to the club
		#[pallet::weight(T::WeightInfo::add_member())]
		#[pallet::call_index(6)]
		pub fn add_member(
			origin: OriginFor<T>,
			club_id: u64,
			requester: T::AccountId,
		) -> DispatchResult {
			let owner = ensure_signed(origin)?;

			let maybe_club = Clubs::<T>::get(club_id);
			if maybe_club.is_none() {
				return Err(Error::<T>::ClubNotFound.into())
			}

			let club = maybe_club.unwrap();
			if club.owner != owner {
				return Err(Error::<T>::NotClubOwner.into())
			}

			let maybe_membership_request = MembershipRequest::<T>::get(&requester, club_id);
			if maybe_membership_request.is_none() {
				return Err(Error::<T>::MembershipRequestNotFound.into())
			}
			let membership_request = maybe_membership_request.unwrap();

			let current_block = <frame_system::Pallet<T>>::block_number();
			let expiry_block = current_block +
				T::BlocksPerYear::get()
					.checked_mul(&T::BlockNumber::from(membership_request.time_in_year))
					.ok_or(DispatchError::Arithmetic(ArithmeticError::Overflow))?;
			let maybe_expirations_per_block = ExpirationsPerBlock::<T>::get(expiry_block);
			let expiry_per_block = if maybe_expirations_per_block.is_none() {
				1
			} else {
				let previous_expirations = maybe_expirations_per_block.unwrap();
				previous_expirations + 1
			};
			ExpirationsPerBlock::<T>::set(expiry_block, Some(expiry_per_block));
			ClubMemberFutureExpirations::<T>::set(
				(expiry_block, expiry_per_block),
				Some((requester.clone(), club_id)),
			);
			MembershipRequest::<T>::remove(&requester, club_id);
			ClubMembership::<T>::set(
				&requester,
				club_id,
				Some(MembershipDetails { is_renewal: membership_request.is_renewal }),
			);

			Self::deposit_event(Event::<T>::MemberAdded {
				club_id,
				member: requester,
				membership_expiry_block: expiry_block,
			});

			Ok(())
		}
	}
}
