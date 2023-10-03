define cargo_update_precise
$(foreach s,$1,$(call _cargo_update_precise,$(subst @, ,$s)))
endef

define _cargo_update_precise
	$(call __cargo_op,update,--workspace -p '$(word 1,$1)' --precise '$(word 2,$1)')

endef

.cargo-update-precise-pre:	prepare
	$(call __cargo_op,update,--workspace)
