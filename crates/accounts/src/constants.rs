use miden_assembly::{
    ast::{Module, ModuleKind},
    DefaultSourceManager, LibraryPath,
};
use miden_lib::transaction::TransactionKernel;
use miden_objects::assembly::Library;

use std::sync::{Arc, LazyLock};

pub static ORACLE_COMPONENT_LIBRARY: LazyLock<Library> = LazyLock::new(|| {
    let assembler = TransactionKernel::assembler().with_debug_mode(true);

    let source_manager = Arc::new(DefaultSourceManager::default());
    let oracle_component_module = Module::parser(ModuleKind::Library)
        .parse_str(
            LibraryPath::new("oracle_component::oracle_module").unwrap(),
            ORACLE_ACCOUNT_CODE,
            &source_manager,
        )
        .unwrap();

    assembler
        .assemble_library([oracle_component_module])
        .expect("assembly should succeed")
});

pub const WRITE_DATA_TX_SCRIPT: &str = r#"
    use.oracle_component::oracle_module

    begin
        push.{4}
        push.{3}
        push.{2}
        push.{1}

        call.oracle_module::write_oracle_data

        dropw dropw dropw dropw

        call.::miden::contracts::auth::basic::auth_tx_rpo_falcon512
        drop
    end
"#;

pub const ORACLE_ACCOUNT_CODE: &str = r#"
    use.std::sys
    use.miden::account

    #! Writes new price data into the oracle's data slots.
    #!
    #! Inputs:  [WORD_1, WORD_2, WORD_3, WORD_4]
    #! Outputs: []
    #!
    export.write_oracle_data
        push.0
        exec.account::set_item
        dropw dropw
        # => [WORD_2, WORD_3, WORD_4]

        push.1
        exec.account::set_item
        dropw dropw
        # => [WORD_3, WORD_4]

        push.2
        exec.account::set_item
        dropw dropw
        # => [WORD_4]

        push.3
        exec.account::set_item
        dropw dropw
        # => []
    end

    #! Gets new price data from the oracle's data slots.
    #!
    #! Inputs:  [storage_slot]
    #! Outputs: [WORD]
    #!
    export.get_item_foreign
        # make this foreign procedure unique to make sure that we invoke the procedure of the 
        # foreign account, not the native one
        push.1 drop
        exec.account::get_item

        # truncate the stack
        exec.sys::truncate_stack
    end
"#;
