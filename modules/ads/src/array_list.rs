use frame_support::{StorageMap, Parameter, StorageValue,debug::native,print};
use sp_runtime::traits::Member;
use codec::{Encode, EncodeLike, Decode, Input, Output};
use frame_support::{
    decl_event, decl_module, decl_storage, decl_error, ensure};

#[derive(Encode, Decode, Debug, PartialEq, Eq)]
pub struct ArrayList<Storage, Value, SizeStorage>(sp_std::marker::PhantomData<(Storage, Value, SizeStorage)>);

impl<Storage, Value, SizeStorage> ArrayList<Storage, Value, SizeStorage> where Value: Parameter + Member + Copy ,Storage: StorageMap<u64, Value, Query=Option<Value>>, SizeStorage: StorageValue<u64> {
    pub fn get(index: &u64)->Option<Value>{
        Storage::get(&index)
    }
    
    pub fn add(value: &Value) -> bool {
        let index:u64 = match SizeStorage::try_get(){
            Ok(i)=> i,
            Err(e)=>0u64
        };
        if let Some(size) = index.checked_add(1) {
            Storage::insert(&index, value);
            SizeStorage::put(size);
            return true;
        }
        false
    }

    pub fn remove(index: &u64) -> bool {
        let mut size:u64 = match SizeStorage::try_get(){
            Ok(i)=>i,
            Err(e)=>0
        };
        if size > 0u64 && *index <= size - 1u64 {
            size = size.clone() - 1;
            let last: Value = Storage::get(size).unwrap(); //last one
            Storage::insert(&index, last);
            Storage::take(&size);
            SizeStorage::put(size);
            return true;
        }
        false
    }

    pub fn size() -> u64{
        match  SizeStorage::try_get(){
            Ok(size)=>size,
            Err(err) =>0u64
        }
    }
}