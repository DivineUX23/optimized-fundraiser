use pinocchio::{
    AccountView, ProgramResult, cpi::{Seed, Signer}, error::ProgramError, sysvars::{Sysvar, clock::Clock}
};

use crate::{state::{Fundraiser, Contributor}, SECONDS_TO_DAYS};

#[inline(always)]
pub fn process_refund_instruction(accounts: &mut [AccountView], data: &[u8]) -> ProgramResult {
    let [
        contributor,
        maker,
        mint_to_raise,
        fundraiser,
        contributor_account,
        contributor_ata,
        vault,
        _system_program,
        _token_program,
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

    let bump = unsafe { *( data.as_ptr() ) };

    let fundraiser_data = Fundraiser::from_account_info(fundraiser)?;

    if fundraiser_data.maker().as_array() != maker.address().as_array() {
        return Err(ProgramError::InvalidArgument);
    }

    let contributor_data = Contributor::from_account_info(contributor_account)?;

    let current_time = Clock::get()?.unix_timestamp;
    if fundraiser_data.duration < ((current_time - fundraiser_data.time_started())/SECONDS_TO_DAYS) as u8 {
        return Err(ProgramError::InvalidArgument);
    }

    let vault_amount = unsafe {(vault.borrow_unchecked().as_ptr().add(64) as *const u64).read_unaligned()};

    if vault_amount >= fundraiser_data.amount_to_raise() {
        return Err(ProgramError::InvalidArgument);
    }

    fundraiser_data.set_current_amount(fundraiser_data.current_amount() - contributor_data.amount());


    let bump_bytes = [bump];
    let signer_seeds = [
        Seed::from(b"fundraiser"),
        Seed::from(maker.address().as_array()),
        Seed::from(bump_bytes.as_ref()),
    ];
    let signer = Signer::from(&signer_seeds);


    pinocchio_token::instructions::Transfer::new(vault, contributor_ata, fundraiser, contributor_data.amount())
        .invoke_signed(&[signer])?;


    contributor.set_lamports(contributor.lamports() + contributor_account.lamports());
    contributor_account.set_lamports(0);

    let _ = contributor_account.close();

    Ok(())
}