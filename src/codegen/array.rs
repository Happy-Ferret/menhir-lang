use std::os::raw::c_uint;
use llvm::prelude::*;
use llvm::core::*;

use ast::{Type, array_type};
use codegen::{const_int, cstr, ValueRef, Context};
use compileerror::{CompileResult, Pos};


#[derive(Debug, Clone)]
pub struct ArrayData
{
    array: LLVMValueRef,
    element_type: Type,
}

impl ArrayData
{
    pub unsafe fn alloc(ctx: &Context, element_type: &Type, len: usize) -> ArrayData
    {
        let typ = LLVMArrayType(ctx.resolve_type(element_type), len as c_uint);
        ArrayData{
            array: ctx.alloc(typ, "array_data"),
            element_type: element_type.clone(),
        }
    }

    pub unsafe fn get_element(&self, ctx: &Context, index: LLVMValueRef) -> ValueRef
    {
        let mut index_expr = vec![const_int(ctx, 0), index];
        let element = LLVMBuildGEP(ctx.builder, self.array, index_expr.as_mut_ptr(), 2, cstr("el"));
        ValueRef::new(element, &self.element_type)
    }
}

#[derive(Debug, Clone)]
pub struct Array
{
    array: LLVMValueRef,
    element_type: Type,
}



impl Array
{
    pub unsafe fn new(arr: LLVMValueRef, element_type: Type) -> Array
    {
        Array{
            array: arr,
            element_type: element_type,
        }
    }

    pub unsafe fn empty(ctx: &Context, pos: Pos) -> CompileResult<Array>
    {
        let element_type = ctx.resolve_type(&Type::Int);
        let slice_type = ctx.resolve_type(&array_type(Type::Int));
        let slice = Array{
            element_type: Type::Int,
            array: ctx.alloc(slice_type, "array"),
        };

        let length_ptr = slice.get_length_ptr(ctx);
        try!(length_ptr.store_direct(ctx, const_int(ctx, 0), pos));

        let offset_ptr = slice.get_offset_ptr(ctx);
        try!(offset_ptr.store_direct(ctx, const_int(ctx, 0), pos));

        let data_ptr = slice.get_data_ptr(ctx);
        try!(data_ptr.store_direct(ctx, LLVMConstPointerNull(LLVMPointerType(element_type, 0)), pos));
        Ok(slice)
    }

    pub unsafe fn init(&mut self, ctx: &Context, len: usize, pos: Pos) -> CompileResult<()>
    {
        // First allocate the storage
        let array_data = ArrayData::alloc(ctx, &self.element_type, len);

        let length_ptr = self.get_length_ptr(ctx);
        try!(length_ptr.store_direct(ctx, const_int(ctx, len as u64), pos));

        let offset_ptr = self.get_offset_ptr(ctx);
        try!(offset_ptr.store_direct(ctx, const_int(ctx, 0), pos));

        let first = array_data.get_element(ctx, const_int(ctx, 0));
        let data_ptr = self.get_data_ptr(ctx);
        try!(data_ptr.store_direct(ctx, first.get(), pos));
        Ok(())
    }

    pub unsafe fn alloc(ctx: &Context, element_type: Type, len: usize, pos: Pos) -> CompileResult<Array>
    {
        let slice_type = ctx.resolve_type(&array_type(element_type.clone()));
        let mut slice = Array{
            element_type: element_type,
            array: ctx.alloc(slice_type, "array"),
        };

        try!(slice.init(ctx, len, pos));
        Ok(slice)
    }


    pub fn get(&self) -> LLVMValueRef {self.array}

    pub unsafe fn head(&self, ctx: &Context) -> ValueRef
    {
        self.get_element(ctx, const_int(ctx, 0))
    }

    pub unsafe fn tail(&self, ctx: &Context, pos: Pos) -> CompileResult<ValueRef>
    {
        self.subslice(ctx, 1, pos)
    }

    pub unsafe fn get_length_ptr(&self, ctx: &Context) -> ValueRef
    {
        ValueRef::Ptr(LLVMBuildStructGEP(ctx.builder, self.array, 1, cstr("length")))
    }

    pub unsafe fn get_offset_ptr(&self, ctx: &Context) -> ValueRef
    {
        ValueRef::Ptr(LLVMBuildStructGEP(ctx.builder, self.array, 2, cstr("offset")))
    }

    pub unsafe fn get_data_ptr(&self, ctx: &Context) -> ValueRef
    {
        ValueRef::Ptr(LLVMBuildStructGEP(ctx.builder, self.array, 0, cstr("data_ptr")))
    }

    pub unsafe fn subslice(&self, ctx: &Context, offset: u64, pos: Pos) -> CompileResult<ValueRef>
    {
        let slice_type = ctx.resolve_type(&array_type(self.element_type.clone()));
        let slice = Array{
            array: ctx.alloc(slice_type, "subslice"),
            element_type: self.element_type.clone(),
        };

        let slice_length_ptr = slice.get_length_ptr(ctx);
        let length_ptr = self.get_length_ptr(ctx);
        let new_length = LLVMBuildSub(ctx.builder, length_ptr.load(ctx.builder), const_int(ctx, offset), cstr("new_length"));
        try!(slice_length_ptr.store_direct(ctx, new_length, pos));

        let slice_offset_ptr = slice.get_offset_ptr(ctx);
        let offset_ptr = self.get_offset_ptr(ctx);
        let new_offset = LLVMBuildAdd(ctx.builder, offset_ptr.load(ctx.builder), const_int(ctx, offset), cstr("new_offset"));
        try!(slice_offset_ptr.store_direct(ctx, new_offset, pos));

        let slice_data_ptr = slice.get_data_ptr(ctx);
        try!(slice_data_ptr.store_direct(ctx, self.get_data_ptr(ctx).load(ctx.builder), pos));
        Ok(ValueRef::Array(slice))
    }

    pub unsafe fn get_element(&self, ctx: &Context, idx: LLVMValueRef) -> ValueRef
    {
        let data_ptr = self.get_data_ptr(ctx);
        let array_ptr = data_ptr.load(ctx.builder);

        let offset_ptr = self.get_offset_ptr(ctx);
        let offset = offset_ptr.load(ctx.builder);
        let pos = LLVMBuildAdd(ctx.builder, offset, idx, cstr("add"));

        let mut index_expr = vec![pos];
        let element = LLVMBuildGEP(ctx.builder, array_ptr, index_expr.as_mut_ptr(), 1, cstr("el"));
        ValueRef::new(element, &self.element_type)
    }

    pub unsafe fn gen_length(&self, ctx: &Context) -> ValueRef
    {
        ValueRef::Const(self.get_length_ptr(ctx).load(ctx.builder))
    }
}
