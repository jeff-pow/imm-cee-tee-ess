import chess
import chess.engine
import chess.pgn

import logging
logging.basicConfig(level=logging.DEBUG)

pgn = open("game.pgn")
game = chess.pgn.read_game(pgn)
base = chess.engine.SimpleEngine.popen_uci("./imm-cee-tee-ess", debug=True)
base.configure({"Threads": 1, "Hash": 32})

# Determine which side the engine is playing
if game.headers["White"] == "imm-cee-tee-ess-dev":
    offset = 0  # Engine is White
else:
    offset = 1  # Engine is Black

board = game.board()

# Iterate over each move in the mainline of the game
for idx, node in enumerate(game.mainline()):
    # Alternate between player's and engine's moves based on the offset
    if (idx % 2) == offset:  # Engine's turn
        comment = node.comment
        print(node.comment)

        node_count = int(comment.split(" ")[-1].rstrip(","))

        result = base.play(board, chess.engine.Limit(nodes=node_count))

        assert result.move == node.move, "Engine move does not match PGN move"
        board.push(result.move)

    else:  # Opponent's turn, push PGN move
        board.push(node.move)

# Perform one final move from the engine after the game ends
result = base.play(board, chess.engine.Limit(time=0.01))
print(result)

base.quit()
