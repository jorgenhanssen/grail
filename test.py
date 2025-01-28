# Simple dummy test training a neural network to predict the score of a board

import pandas as pd
import numpy as np
from sklearn.model_selection import train_test_split
from tensorflow.keras.models import Sequential
from tensorflow.keras.layers import Dense
from tensorflow.keras.regularizers import l2


df = pd.read_csv("samples.csv", header=None, dtype=np.float32)

X = df.iloc[:, 1:].values
y = df.iloc[:, 0].values

# Optional: Split data
X_train, X_test, y_train, y_test = train_test_split(
    X, y, test_size=0.2, random_state=42
)

model = Sequential()
model.add(Dense(512, activation="relu", input_shape=(X.shape[1],)))
model.add(Dense(256, activation="relu"))
model.add(Dense(256, activation="relu"))
model.add(Dense(1, activation="tanh"))

model.compile(optimizer="adam", loss="mean_squared_error", metrics=["mse"])

history = model.fit(X_train, y_train, validation_split=0.2, epochs=10, batch_size=128)

test_loss, test_mse = model.evaluate(X_test, y_test, verbose=0)
print(f"Test MSE: {test_mse}")

predictions = model.predict(X_test)

# Display first 100 test samples with their true labels and predictions
for i in range(min(1000, len(y_test))):
    print(f"Sample {i + 1}: True Label = {y_test[i]}, Prediction = {predictions[i][0]}")
