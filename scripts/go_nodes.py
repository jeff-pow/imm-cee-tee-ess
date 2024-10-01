# Credit to Algorhythm-sxv for this
import chess
import chess.engine
import chess.pgn
import itertools

import logging
logging.basicConfig(level=logging.DEBUG)

pgn = open("game.pgn")
game = chess.pgn.read_game(pgn)
base = chess.engine.SimpleEngine.popen_uci("./imm-cee-tee-ess", debug=True)
base.configure({"Threads": 1, "Hash": 32})

if game.headers["White"] == "imm-cee-tee-ess-dev":
    print('Dev is white')
    offset = 0
else:
    print('Dev is black')
    offset = 1

board = game.board()
for node in itertools.islice(game.mainline(), offset, None, 2):
    comment = node.comment
    node_count = int(comment.split(' ')[3].rstrip(','))
    result = base.play(board, chess.engine.Limit(nodes=node_count))
    print(node_count, node.move, result.move)
    assert (result.move == node.move)
    board.push(result.move)

result = base.play(board, chess.engine.Limit(time=1))

print(result)

base.quit()
