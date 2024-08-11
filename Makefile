EXE = imm-cee-tee-ess
LXE = imm-cee-tee-ess

ifeq ($(OS),Windows_NT)
	NAME := $(EXE).exe
else
	NAME := $(LXE)
endif

openbench:
	cargo rustc --release -- -C target-cpu=native --emit link=$(NAME)

bench:
	cargo rustc --release -- -C target-cpu=native --emit link=$(NAME)
	./$(NAME) bench
