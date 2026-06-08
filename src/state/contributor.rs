use pinocchio::{AccountView, error::ProgramError};

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Contributor {
    amount: [u8; 8]
}

impl Contributor {
    pub const LEN: usize = 8;

    #[inline(always)]
    pub fn from_account_info(account_info: &mut AccountView) -> Result<&mut Self, ProgramError> {
        let data = unsafe { account_info.borrow_unchecked_mut() };
        if data.len() != Self::LEN {
            return Err(ProgramError::InvalidAccountData);
        }
        Ok(unsafe { &mut *(data.as_mut_ptr() as *mut Self) })
    }

    #[inline(always)]
    pub fn amount(&self) -> u64 {
        unsafe { ( self.amount.as_ptr() as *const u64 ).read_unaligned() }
    }

    #[inline(always)]
    pub fn set_amount(&mut self, amount: u64) {
        unsafe { ( self.amount.as_mut_ptr() as *mut u64 ).write_unaligned(amount) }
    }
}