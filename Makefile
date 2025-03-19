all: comp1

j.%: prj4
	docker compose -f jeremy_tests/$*.yml up

comp%: prj4
	docker compose -f testcases/docker-compose-testcase-$*.yml up

prj4:
	docker build . -t $@

.PHONY: teardown%
teardown%:
	docker compose -f testcases/docker-compose-testcase-$*.yml down
