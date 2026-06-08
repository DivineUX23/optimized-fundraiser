use pinocchio::{
    AccountView, ProgramResult, cpi::{Seed, Signer}, error::ProgramError, sysvars::{Sysvar, clock::Clock}
};
use pinocchio_system::instructions::CreateAccount;

use crate::{constants::{CONTRIBUTOR_RENT_EXEMPT, MAX_CONTRIBUTION_DENOMINATOR, SECONDS_TO_DAYS}, state::{Contributor, Fundraiser}};


#[inline(always)]
pub fn process_contribute_instruction(accounts: &mut [AccountView], data: &[u8]) -> ProgramResult {
    let [
        contributor,
        mint_to_raise,
        fundraiser,
        contributor_account,
        contributor_ata,
        vault,
        _system_program,
        _token_program,
        _associated_token_program
    ] = accounts 
    else {
        return Err(ProgramError::NotEnoughAccountKeys)
    };

    {
        let ata_state = unsafe { contributor_ata.borrow_unchecked() };

        let ata_mint = unsafe { &*(ata_state.as_ptr() as *const [u8; 32]) };

        let ata_owner = unsafe { &*(ata_state.as_ptr().add(32) as *const [u8; 32]) };


        if ata_owner != contributor.address().as_array() {
            return Err(ProgramError::IllegalOwner);
        }

        if ata_mint != mint_to_raise.address().as_array() {
            return Err(ProgramError::InvalidAccountData);
        }
    }


    let amount = unsafe { ( data.as_ptr() as *const u64 ).read_unaligned() };

    let bump = unsafe { *( data.as_ptr().add(8) ) };

    let fundraiser_data = Fundraiser::from_account_info(fundraiser)?;

    let max_contribution = fundraiser_data.amount_to_raise() / MAX_CONTRIBUTION_DENOMINATOR;

    if amount == 0 || amount > max_contribution {
        return Err(ProgramError::InvalidArgument);
    }

    let current_time = Clock::get()?.unix_timestamp;
    let days = ((current_time - fundraiser_data.time_started())/SECONDS_TO_DAYS) as u8;
    if fundraiser_data.duration >= days {
        return Err(ProgramError::InvalidArgument);
    }


    if !contributor_account.owned_by(&crate::ID) {

        let bump_bytes = [bump];
        let signer_seeds = [
            Seed::from(b"contributor"),
            Seed::from(contributor.address().as_array()),
            Seed::from(bump_bytes.as_ref()),
        ];
        let signer = Signer::from(&signer_seeds);

        CreateAccount {
            from: contributor,
            to: contributor_account,
            lamports: CONTRIBUTOR_RENT_EXEMPT,
            space: Contributor::LEN as u64,
            owner: &crate::ID
        }
        .invoke_signed(&[signer])?;

        let contributor_data = Contributor::from_account_info(contributor_account)?;

        contributor_data.set_amount(amount);

    } else {

        let contributor_data = Contributor::from_account_info(contributor_account)?;
        
        let fund_amount = contributor_data.amount();

        if fund_amount + amount > max_contribution {
            return Err(ProgramError::InvalidArgument);
        }

        contributor_data.set_amount(fund_amount + amount);

    }

    pinocchio_token::instructions::Transfer::new(contributor_ata, vault, contributor, amount)
        .invoke()?;

    let fund_amount = fundraiser_data.current_amount();
    fundraiser_data.set_current_amount(fund_amount + amount);

    Ok(())
}