EXE = imm-cee-tee-ess

ifeq ($(OS),Windows_NT)
	NAME := $(EXE).exe
else
	NAME := $(EXE)
endif

openbench:
	cargo rustc --bin imm-cee-tee-ess --release -- -C target-cpu=native --emit link=$(NAME)

bench:
	cargo rustc --bin imm-cee-tee-ess --release -- -C target-cpu=native --emit link=$(NAME)
	./$(NAME) bench
