#![allow(clippy::type_complexity)]

use frame_support::{traits::OnRuntimeUpgrade, weights::Weight};
use log::{info, warn};

pub mod new_core_account_derivation {
    use super::*;
    use crate::{common_types::CommonId, AccountId, Identity, Runtime, RuntimeOrigin, Vec};
    use frame_support::dispatch::GetDispatchInfo;
    use pallet_inv4::{
        account_derivation::CoreAccountDerivation, CoreInfoOf, Pallet as INV4Pallet,
    };
    use sp_std::boxed::Box;

    fn get_old_accounts() -> Vec<(AccountId, CommonId)> {
        pallet_inv4::CoreByAccount::<Runtime>::iter().collect()
    }

    fn migrate_inv4_storages(old_accounts: Vec<(AccountId, CommonId)>) {
        old_accounts.iter().for_each(|(old_acc, core_id)| {
            let new_account =
                <INV4Pallet<Runtime> as CoreAccountDerivation<Runtime>>::derive_core_account(
                    *core_id,
                );

            pallet_inv4::CoreByAccount::<Runtime>::swap(old_acc, new_account);
        });

        pallet_inv4::CoreStorage::<Runtime>::translate(
            |core_id, core_data: CoreInfoOf<Runtime>| {
                let mut new_core = core_data;

                new_core.account =
                    <INV4Pallet<Runtime> as CoreAccountDerivation<Runtime>>::derive_core_account(
                        core_id,
                    );

                Some(new_core)
            },
        );
    }

    fn migrate_staking_storages(old_accounts: Vec<(AccountId, CommonId)>) {
        old_accounts.iter().for_each(|(old_acc, this_core_id)| {
            let new_account =
                <INV4Pallet<Runtime> as CoreAccountDerivation<Runtime>>::derive_core_account(
                    *this_core_id,
                );

            pallet_ocif_staking::pallet::Ledger::<Runtime>::swap(old_acc, new_account.clone());

            pallet_inv4::CoreStorage::<Runtime>::iter_keys().for_each(|staking_core_id| {
                pallet_ocif_staking::pallet::GeneralStakerInfo::<Runtime>::swap(
                    staking_core_id,
                    old_acc,
                    staking_core_id,
                    new_account.clone(),
                );
            });
        });
    }

    fn migrate_balances_storages(old_accounts: Vec<(AccountId, CommonId)>) {
        old_accounts.iter().for_each(|(old_acc, this_core_id)| {
            let new_account =
                <INV4Pallet<Runtime> as CoreAccountDerivation<Runtime>>::derive_core_account(
                    *this_core_id,
                );

            pallet_balances::Account::<Runtime>::swap(old_acc, new_account.clone());
            pallet_balances::Freezes::<Runtime>::swap(old_acc, new_account.clone());
            pallet_balances::Holds::<Runtime>::swap(old_acc, new_account.clone());
            pallet_balances::Locks::<Runtime>::swap(old_acc, new_account.clone());
            pallet_balances::Reserves::<Runtime>::swap(old_acc, new_account.clone());

            frame_system::Account::<Runtime>::swap(old_acc, new_account);
        });
    }

    fn clear_identities(
        old_accounts: Vec<(AccountId, CommonId)>,
    ) -> Vec<(
        AccountId,
        Option<
            pallet_identity::Registration<
                crate::Balance,
                crate::MaxRegistrars,
                crate::MaxAdditionalFields,
            >,
        >,
    )> {
        old_accounts
            .iter()
            .map(|(old_acc, this_core_id)| {
                let new_account =
                    <INV4Pallet<Runtime> as CoreAccountDerivation<Runtime>>::derive_core_account(
                        *this_core_id,
                    );

                let maybe_identity = Identity::identity(old_acc);

                if maybe_identity.is_some() {
                    let _ = Identity::clear_identity(RuntimeOrigin::signed(old_acc.clone()));
                }

                (new_account, maybe_identity)
            })
            .collect::<Vec<(
                AccountId,
                Option<
                    pallet_identity::Registration<
                        crate::Balance,
                        crate::MaxRegistrars,
                        crate::MaxAdditionalFields,
                    >,
                >,
            )>>()
    }

    fn set_new_identities(
        new_identities: Vec<(
            AccountId,
            Option<
                pallet_identity::Registration<
                    crate::Balance,
                    crate::MaxRegistrars,
                    crate::MaxAdditionalFields,
                >,
            >,
        )>,
    ) -> Weight {
        let mut weight: Weight = Default::default();

        new_identities
            .into_iter()
            .for_each(|(new_account, maybe_identity)| {
                if let Some(identity) = maybe_identity {
                    let _ = Identity::set_identity(
                        RuntimeOrigin::signed(new_account.clone()),
                        Box::new(identity.info.clone()),
                    );

                    weight.saturating_accrue(
                        pallet_identity::Call::<Runtime>::set_identity {
                            info: Box::new(identity.info),
                        }
                        .get_dispatch_info()
                        .weight,
                    );
                }
            });

        return weight;
    }

    pub struct MigrateToNewDerivation;
    impl OnRuntimeUpgrade for MigrateToNewDerivation {
        fn on_runtime_upgrade() -> Weight {
            let mut weight = <Runtime as frame_system::Config>::DbWeight::get().reads(1);

            let spec = crate::System::runtime_version().spec_version;

            if spec == 21 {
                weight
                    .saturating_accrue(<Runtime as frame_system::Config>::DbWeight::get().reads(1));
                let old_accounts = get_old_accounts();
                let old_accounts_len = old_accounts.len() as u64;

                if let Some((acc, id)) = old_accounts.first() {
                    let new_acc =
                <INV4Pallet<Runtime> as CoreAccountDerivation<Runtime>>::derive_core_account(
                    *id,
                );

                    if *acc != new_acc {
                        weight.saturating_accrue(
                            <Runtime as frame_system::Config>::DbWeight::get().reads_writes(2, 2)
                                * old_accounts_len,
                        );
                        migrate_inv4_storages(old_accounts.clone());

                        weight.saturating_accrue(
                            <Runtime as frame_system::Config>::DbWeight::get().reads_writes(3, 2)
                                * old_accounts_len,
                        );
                        migrate_staking_storages(old_accounts.clone());

                        let clear_identities_weight =
                            <Runtime as frame_system::Config>::DbWeight::get()
                                .reads(old_accounts_len)
                                + (pallet_identity::Call::<Runtime>::clear_identity {}
                                    .get_dispatch_info()
                                    .weight
                                    * old_accounts_len);
                        weight.saturating_accrue(clear_identities_weight);
                        let new_identities = clear_identities(old_accounts.clone());

                        weight.saturating_accrue(
                            <Runtime as frame_system::Config>::DbWeight::get().reads_writes(6, 6)
                                * old_accounts_len,
                        );
                        migrate_balances_storages(old_accounts.clone());

                        let set_identities_weight = set_new_identities(new_identities.clone());

                        weight.saturating_accrue(set_identities_weight);

                        info!("applied successfully");
                    }
                }
            } else {
                warn!("Skipping, should be removed");
            }

            return weight;
        }
    }
}
