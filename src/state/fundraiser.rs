use pinocchio::{
    AccountView, Address, error::ProgramError
};

#[repr(C)]
//#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Fundraiser {
    maker: [u8; 32],
    mint_to_raise: [u8; 32],
    amount_to_raise: [u8; 8],
    current_amount: [u8; 8],
    time_started: [u8; 8],
    pub duration: u8,
    pub bump: u8,
}

impl Fundraiser {
    pub const LEN: usize = 90;
    /*
    #[inline(always)]
    pub unsafe fn from_account_info_unchecked(account: &mut AccountView) -> &mut Self {
        &mut *(account.borrow_unchecked_mut().as_mut_ptr() as *mut Self)
    }
    */

    #[inline(always)]
    pub fn from_account_info(account_info: &mut AccountView) -> Result<&mut Self, ProgramError> {
        let data = unsafe { account_info.borrow_unchecked_mut() };
        if data.len() != Self::LEN {
            return Err(ProgramError::InvalidAccountData);
        }
        Ok(unsafe { &mut *(data.as_mut_ptr() as *mut Self) })
    }

    #[inline(always)]
    pub fn maker(&self) -> &Address {
        unsafe { &*(self.maker.as_ptr() as *const Address) }
    }

    /*
    #[inline(always)]
    pub fn set_maker(&mut self, maker: &Address) {
        self.maker.copy_from_slice(maker.as_ref());
    }
    */

    #[inline(always)]
    pub fn set_maker(&mut self, maker: &Address) {
        unsafe {
            core::ptr::copy_nonoverlapping(
                maker.as_ref().as_ptr(), 
                self.maker.as_mut_ptr(), 
                32
            );
        }
    }

    #[inline(always)]
    pub fn mint_to_raise(&self) -> &Address {
        unsafe { &*(self.mint_to_raise.as_ptr() as *const Address) }
    }


    #[inline(always)]
    pub fn set_mint_to_raise(&mut self, mint: &Address) {
        unsafe {
            core::ptr::copy_nonoverlapping(
                mint.as_ref().as_ptr(), 
                self.mint_to_raise.as_mut_ptr(), 
            32);
        }

    }


    #[inline(always)]
    pub fn amount_to_raise(&self) -> u64 {
        unsafe { ( self.amount_to_raise.as_ptr() as *const u64 ).read_unaligned() }
    }

    #[inline(always)]
    pub fn set_amount_to_raise(&mut self, amount: u64) {
        unsafe { ( self.amount_to_raise.as_mut_ptr() as *mut u64 ).write_unaligned(amount); }
    }

    #[inline(always)]
    pub fn current_amount(&self) -> u64 {
        unsafe { ( self.current_amount.as_ptr() as *const u64 ).read_unaligned() }
    }

    #[inline(always)]
    pub fn set_current_amount(&mut self, amount: u64) {
        unsafe { ( self.current_amount.as_ptr() as *mut u64 ).write_unaligned(amount) }
    }

    #[inline(always)]
    pub fn time_started(&self) -> i64 {
        unsafe { ( self.time_started.as_ptr() as *mut i64 ).read_unaligned() }
    }

    #[inline(always)]
    pub fn set_time_started(&mut self, time: i64) {
        unsafe { ( self.time_started.as_ptr() as *mut i64 ).write_unaligned(time) }
    }

}