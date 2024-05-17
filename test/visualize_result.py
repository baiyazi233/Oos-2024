import json
import sys

TEST_SKIPED = 0
TEST_FAILED = 1
TEST_PASSED = 2

def get_prefix(status: int):
    if status == TEST_FAILED:
        # Red "Failed"
        return "\033[31mFailed\033[0m"
    elif status == TEST_SKIPED:
        # Bright Yellow "Half"
        return "\033[93mSkiped\033[0m"
    elif status == TEST_PASSED:
        # Green "Passed"
        return "\033[32mPassed\033[0m"

def visualize(data):
    total_tests = len(data)
    failed = 0
    skiped = 0
    passed = 0

    total_scores = 0
    score = 0
    for test in data:
        name = test['name']
        num_total = test['all']
        num_passed = test['passed']
        judgements = len(test['results'])

        total_scores += num_total
        score += num_passed

        status = 0

        if judgements == 0:
            skiped += 1
            status = TEST_SKIPED
        elif num_passed == num_total:
            passed += 1
            status = TEST_PASSED
        else:
            failed += 1
            status = TEST_FAILED

        print(f"  {get_prefix(status)} {name} [{num_passed}/{num_total}]")
    
    print()
    print(f"Total tests: {total_tests}")
    print(f"    Skipped: {skiped}")
    print(f"    Passed: {passed}")
    print(f"    Failed: {failed}")
    print()
    print(f"Scores: {score}/{total_scores}")
    print()
    print("Raw test result and kernel output will be uploaded to the artifacts")

def read_output(file_name: str):
    with open(file_name, "r") as file:
        return file.read()
def print_pattern():
    pattern = """
 __    __   ______   ________  _______            ______                      
/  |  /  | /      \ /        |/       \          /      \                     
$$ |  $$ |/$$$$$$  |$$$$$$$$/ $$$$$$$  |        /$$$$$$  |  ______    _______ 
$$ |  $$ |$$ \__$$/    $$ |   $$ |__$$ | ______ $$ |  $$ | /      \  /       |
$$ |  $$ |$$      \    $$ |   $$    $$&lt; /      |$$ |  $$ |/$$$$$$  |/$$$$$$$/ 
$$ |  $$ | $$$$$$  |   $$ |   $$$$$$$  |$$$$$$/ $$ |  $$ |$$ |  $$ |$$      \ 
$$ \__$$ |/  \__$$ |   $$ |   $$ |__$$ |        $$ \__$$ |$$ \__$$ | $$$$$$  |
$$    $$/ $$    $$/    $$ |   $$    $$/         $$    $$/ $$    $$/ /     $$/ 
 $$$$$$/   $$$$$$/     $$/    $$$$$$$/           $$$$$$/   $$$$$$/  $$$$$$$/  
                                                                              
                                                                              
                                                                              
 ________                     __                                              
/        |                   /  |                                             
$$$$$$$$/______    _______  _$$ |_                                            
   $$ | /      \  /       |/ $$   |                                           
   $$ |/$$$$$$  |/$$$$$$$/ $$$$$$/                                            
   $$ |$$    $$ |$$      \   $$ | __                                          
   $$ |$$$$$$$$/  $$$$$$  |  $$ |/  |                                         
   $$ |$$       |/     $$/   $$  $$/                                          
   $$/  $$$$$$$/ $$$$$$$/     $$$$/                                           
                                                                              
                                                                              
                                                                              
"""
    print(pattern)

if __name__ == '__main__':
    file_name = sys.argv[1]
    output = read_output(file_name)
    data = json.loads(output)
    print_pattern()
    print("Result:")
    visualize(data)
